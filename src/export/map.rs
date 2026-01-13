//! Global dwarf fortress map intermediate storage between Dwarf Fortress and voxels

use crate::{
    coords::{WithBlockCoords, WithBoundingBox},
    direction::{DirectionFlat, Neighbouring, Neighbouring8Flat, NeighbouringFlat},
    export::{BlockTileExt, DFContext},
    rfr::{self, BlockTile, BuildingExt, BuildingFlags},
    DFMapCoords, WithDFCoords,
};
use dfhack_remote::{BuildingInstance, Engraving, MapBlock};
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Default)]
pub struct LevelData<'a> {
    pub blocks: Vec<&'a MapBlock>,
    pub buildings: Vec<&'a BuildingInstance>,
}

/// Intermediary format between DF and voxels
#[derive(Default)]
pub struct Map<'a> {
    /// The map stored by layers
    pub levels: HashMap<i32, LevelData<'a>>,
    /// Quick access to the occupancy data of each tile, for connectivity checks
    pub occupancy: HashMap<DFMapCoords, Occupancy<'a>>,
    /// True if the building where added already, they are streamed multiple times
    buildings_added: bool,
}

#[derive(Default)]
pub struct Occupancy<'a> {
    pub block_tile: Option<BlockTile<'a>>,
    pub buildings: Vec<&'a BuildingInstance>,
    pub engraving: Option<Engraving>,
}

impl<'a> Map<'a> {
    pub fn add_block(&mut self, block: &'a MapBlock, context: &'a DFContext) {
        if !self.buildings_added {
            self.add_buildings(&block.buildings);
        }
        let level = block.block_coords().z;
        self.levels.entry(level).or_default().blocks.push(block);

        for tile in rfr::TileIterator::new(block, &context.tile_types) {
            let coords = tile.global_coords();
            self.occupancy.entry(coords).or_default().block_tile = Some(tile);
        }
    }

    pub fn add_engraving(&mut self, engraving: Engraving) {
        let coords = engraving.coords();
        self.occupancy.entry(coords).or_default().engraving = Some(engraving);
    }

    fn add_buildings(&mut self, buildings: &'a Vec<BuildingInstance>) {
        for building in buildings {
            if building.room.is_some() {
                continue;
            }

            if !building
                .building_flags_typed()
                .contains(BuildingFlags::EXISTS)
            {
                continue;
            }

            self.levels
                .entry(building.bounding_box().origin().z)
                .or_default()
                .buildings
                .push(building);

            let bounding_box = building.bounding_box();
            for x in bounding_box.x.clone() {
                for y in bounding_box.y.clone() {
                    for z in bounding_box.z.clone() {
                        self.occupancy
                            .entry(DFMapCoords::new(x, y, z))
                            .or_default()
                            .buildings
                            .push(building);
                    }
                }
            }
        }
        self.buildings_added = true;
    }

    /// Compute a given function for all the neighbours including above and below
    pub fn neighbouring<F, T>(&self, coords: DFMapCoords, func: F) -> Neighbouring<T>
    where
        F: Fn(&Occupancy<'a>) -> T,
    {
        let default = Occupancy::default();
        Neighbouring::new(|direction| {
            let neighbour = coords + direction;
            func(self.occupancy.get(&neighbour).unwrap_or(&default))
        })
    }

    /// Compute a given function for all the neighbours on the same plane
    pub fn neighbouring_flat<F, T>(&self, coords: DFMapCoords, func: F) -> NeighbouringFlat<T>
    where
        F: Fn(&Occupancy<'a>) -> T,
    {
        let default = Occupancy::default();
        NeighbouringFlat::new(|direction| {
            let neighbour = coords + direction;
            func(self.occupancy.get(&neighbour).unwrap_or(&default))
        })
    }

    /// Compute a given function for all the neighbours on the same plane
    pub fn neighbouring_8flat<F, T>(&self, coords: DFMapCoords, func: F) -> Neighbouring8Flat<T>
    where
        F: Fn(&Occupancy<'a>) -> T,
    {
        let default = Occupancy::default();
        Neighbouring8Flat::new(|direction| {
            let neighbour = coords + direction;
            func(self.occupancy.get(&neighbour).unwrap_or(&default))
        })
    }

    /// Find the most "wally" direction, ie the direction to put furniture against
    pub fn wall_direction(&self, coords: DFMapCoords) -> DirectionFlat {
        let z = coords.z;
        // there's likely a nice way to write that
        // N, E, S, W
        const N: usize = 0;
        const E: usize = 1;
        const S: usize = 2;
        const W: usize = 3;
        let mut wallyness = [0, 0, 0, 0];
        for x in -1..=1 {
            for y in -1..=1 {
                // increase the "wallyness" of a direction by 1 for corners and by 4 for direct contact
                let wally = self
                    .occupancy
                    .get(&DFMapCoords::new(coords.x + x, coords.y + y, z))
                    .is_some_and(|tile| {
                        tile.block_tile.as_ref().is_some_and(|tile| tile.is_wall())
                    });
                if wally {
                    if x == -1 {
                        wallyness[W] += 1;
                        if y == 0 {
                            wallyness[W] += 3;
                        }
                    }

                    if x == 1 {
                        wallyness[E] += 1;
                        if y == 0 {
                            wallyness[E] += 3;
                        }
                    }

                    if y == -1 {
                        wallyness[N] += 1;
                        if x == 0 {
                            wallyness[N] += 3;
                        }
                    }

                    if y == 1 {
                        wallyness[S] += 1;
                        if x == 0 {
                            wallyness[S] += 3;
                        }
                    }
                }
            }
        }

        match wallyness.iter().position_max().unwrap() {
            N => DirectionFlat::North,
            E => DirectionFlat::East,
            S => DirectionFlat::South,
            W => DirectionFlat::West,
            _ => unreachable!(),
        }
    }
}
