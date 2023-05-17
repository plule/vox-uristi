use crate::{
    building::{BuildingInstanceExt, BuildingType},
    direction::{DirectionFlat, Neighbouring, Neighbouring8Flat, NeighbouringFlat},
    rfr::{self, BlockTile},
    tile::BlockTileExt,
    Coords, IsSomeAnd, WithCoords,
};
use dfhack_remote::{BuildingInstance, Coord, FlowInfo, MapBlock, TiletypeList};
use itertools::Itertools;
use std::{collections::HashMap, ops::Add};

/// Intermediary format between DF and voxels
#[derive(Default)]
pub struct Map<'a> {
    pub tiles: HashMap<Coords, BlockTile<'a>>,
    pub buildings: HashMap<Coords, Vec<&'a BuildingInstance>>,
    pub flows: HashMap<Coords, &'a FlowInfo>,
}

impl<'a> Map<'a> {
    pub fn add_block(&mut self, block: &'a MapBlock, tiletypes: &'a TiletypeList) {
        for flow in &block.flows {
            self.flows.insert(flow.coords(), flow);
        }
        for tile in rfr::TileIterator::new(block, tiletypes) {
            self.tiles.insert(tile.coords(), tile);
        }

        for building in &block.buildings {
            if building.building_type() != BuildingType::Unknown {
                self.buildings
                    .entry(building.origin())
                    .or_default()
                    .push(building);
            }
        }
    }

    pub fn remove_overlapping_floors(&mut self) {
        for buildings in self.buildings.values() {
            for building in buildings {
                if building.is_floor() {
                    let bounding_box = building.bounding_box();
                    for x in bounding_box.x.clone() {
                        for y in bounding_box.y.clone() {
                            for z in bounding_box.z.clone() {
                                self.tiles.remove(&Coords::new(x, y, z));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Compute a given function for all the neighbours including above and below
    pub fn neighbouring<F, T>(&self, coords: Coords, func: F) -> Neighbouring<T>
    where
        F: Fn(Option<&BlockTile<'a>>, &Vec<&'a BuildingInstance>) -> T,
    {
        let empty_vec = vec![];
        Neighbouring::new(|direction| {
            let neighbour = coords + direction;
            func(
                self.tiles.get(&neighbour),
                self.buildings.get(&neighbour).unwrap_or(&empty_vec),
            )
        })
    }

    /// Compute a given function for all the neighbours on the same plane
    pub fn neighbouring_flat<F, T>(&self, coords: Coords, func: F) -> NeighbouringFlat<T>
    where
        F: Fn(Option<&BlockTile<'a>>, &Vec<&'a BuildingInstance>) -> T,
    {
        let empty_vec = vec![];
        NeighbouringFlat::new(|direction| {
            let neighbour = coords + direction;
            func(
                self.tiles.get(&neighbour),
                self.buildings.get(&neighbour).unwrap_or(&empty_vec),
            )
        })
    }

    /// Compute a given function for all the neighbours on the same plane
    pub fn neighbouring_8flat<F, T>(&self, coords: Coords, func: F) -> Neighbouring8Flat<T>
    where
        F: Fn(Option<&BlockTile<'a>>, &Vec<&'a BuildingInstance>) -> T,
    {
        let empty_vec = vec![];
        Neighbouring8Flat::new(|direction| {
            let neighbour = coords + direction;
            func(
                self.tiles.get(&neighbour),
                self.buildings.get(&neighbour).unwrap_or(&empty_vec),
            )
        })
    }

    /// Find the most "wally" direction, ie the direction to put furniture against
    pub fn wall_direction(&self, coords: Coords) -> DirectionFlat {
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
                    .get(&Coords::new(coords.x + x, coords.y + y, z))
                    .some_and(|tile| tile.is_wall());
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

impl Coords {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl From<Coord> for Coords {
    fn from(value: Coord) -> Self {
        Self {
            x: value.x(),
            y: value.y(),
            z: value.z(),
        }
    }
}

impl From<&Coord> for Coords {
    fn from(value: &Coord) -> Self {
        Self {
            x: value.x(),
            y: value.y(),
            z: value.z(),
        }
    }
}

impl Add<Coords> for Coords {
    type Output = Coords;

    fn add(self, rhs: Coords) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<'a> Add<Coords> for &'a Coords {
    type Output = Coords;

    fn add(self, rhs: Coords) -> Self::Output {
        Coords::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}
