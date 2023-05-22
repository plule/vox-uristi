use crate::{
    building::BuildingInstanceExt,
    context::DFContext,
    direction::{DirectionFlat, Neighbouring, Neighbouring8Flat, NeighbouringFlat},
    rfr::{self, BlockTile},
    tile::BlockTileExt,
    voxel::FromPrefab,
    DFCoords, IsSomeAnd, WithDFCoords,
};
use dfhack_remote::{BuildingInstance, FlowInfo, MapBlock};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

/// Intermediary format between DF and voxels
#[derive(Default)]
pub struct Map<'a> {
    pub tiles: HashMap<DFCoords, Tile<'a>>,
    pub with_building: HashSet<DFCoords>,
}

#[derive(Default)]
pub struct Tile<'a> {
    pub block_tile: Option<BlockTile<'a>>,
    pub buildings: Vec<&'a BuildingInstance>,
    pub flows: Vec<&'a FlowInfo>,
}

impl<'a> Map<'a> {
    pub fn add_block(&mut self, block: &'a MapBlock, context: &'a DFContext) {
        for flow in &block.flows {
            self.tiles
                .entry(flow.coords())
                .or_default()
                .flows
                .push(flow);
        }
        for tile in rfr::TileIterator::new(block, &context.tile_types) {
            let coords = tile.coords();
            self.tiles.entry(coords).or_default().block_tile = Some(tile);
        }

        for building in &block.buildings {
            if building.room.is_none() {
                self.tiles
                    .entry(building.origin())
                    .or_default()
                    .buildings
                    .push(building);

                let bounding_box = building.bounding_box();
                for x in bounding_box.x.clone() {
                    for y in bounding_box.y.clone() {
                        for z in bounding_box.z.clone() {
                            self.with_building.insert(DFCoords::new(x, y, z));
                        }
                    }
                }
            }
        }
    }

    pub fn remove_overlapping_floors(&mut self, context: &DFContext) {
        let mut coords = Vec::new();
        for tile in self.tiles.values() {
            for building in &tile.buildings {
                if building.is_floor(context) {
                    let bounding_box = building.bounding_box();
                    for x in bounding_box.x.clone() {
                        for y in bounding_box.y.clone() {
                            for z in bounding_box.z.clone() {
                                coords.push(DFCoords::new(x, y, z));
                            }
                        }
                    }
                }
            }
        }

        for coord in coords {
            if let Some(tile) = self.tiles.get_mut(&coord) {
                // TODO: we are also erasing the flows here, would be good not to
                tile.block_tile = None;
            }
        }
    }

    /// Compute a given function for all the neighbours including above and below
    pub fn neighbouring<F, T>(&self, coords: DFCoords, func: F) -> Neighbouring<T>
    where
        F: Fn(&Tile<'a>) -> T,
    {
        let default = Tile::default();
        Neighbouring::new(|direction| {
            let neighbour = coords + direction;
            func(self.tiles.get(&neighbour).unwrap_or(&default))
        })
    }

    /// Compute a given function for all the neighbours on the same plane
    pub fn neighbouring_flat<F, T>(&self, coords: DFCoords, func: F) -> NeighbouringFlat<T>
    where
        F: Fn(&Tile<'a>) -> T,
    {
        let default = Tile::default();
        NeighbouringFlat::new(|direction| {
            let neighbour = coords + direction;
            func(self.tiles.get(&neighbour).unwrap_or(&default))
        })
    }

    /// Compute a given function for all the neighbours on the same plane
    pub fn neighbouring_8flat<F, T>(&self, coords: DFCoords, func: F) -> Neighbouring8Flat<T>
    where
        F: Fn(&Tile<'a>) -> T,
    {
        let default = Tile::default();
        Neighbouring8Flat::new(|direction| {
            let neighbour = coords + direction;
            func(self.tiles.get(&neighbour).unwrap_or(&default))
        })
    }

    /// Find the most "wally" direction, ie the direction to put furniture against
    pub fn wall_direction(&self, coords: DFCoords) -> DirectionFlat {
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
                    .tiles
                    .get(&DFCoords::new(coords.x + x, coords.y + y, z))
                    .some_and(|tile| tile.block_tile.some_and(|tile| tile.is_wall()));
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
