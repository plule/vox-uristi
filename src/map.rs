use crate::{
    building::{Building, BuildingType},
    direction::{DirectionFlat, Neighbouring, NeighbouringFlat},
    flow::FlowExtensions,
    rfr,
    tile::Tile,
};
use dfhack_remote::{Coord, FlowInfo, MapBlock, TiletypeList};
use itertools::Itertools;
use std::{collections::HashMap, fmt::Display, ops::Add};

/// Intermediary format between DF and voxels
#[derive(Default)]
pub struct Map<'a> {
    pub tiles: HashMap<Coords, Tile<'a>>,
    pub buildings: HashMap<Coords, Vec<Building<'a>>>,
    pub flows: HashMap<Coords, &'a FlowInfo>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Coords {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Display for Coords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}

pub trait IsSomeAnd<T> {
    fn some_and(self, f: impl FnOnce(T) -> bool) -> bool;
}

impl<T> IsSomeAnd<T> for Option<T> {
    fn some_and(self, f: impl FnOnce(T) -> bool) -> bool {
        match self {
            None => false,
            Some(x) => f(x),
        }
    }
}

impl<'a> Map<'a> {
    pub fn add_block(&mut self, block: &'a MapBlock, tiletypes: &'a TiletypeList) {
        for building in &block.buildings {
            let building = Building(building);
            if building.building_type() != BuildingType::Unknown {
                self.buildings
                    .entry(building.origin())
                    .or_default()
                    .push(building);
            }
        }

        for flow in &block.flows {
            self.flows.insert(flow.coords(), flow);
        }
        for tile in rfr::TileIterator::new(block, tiletypes) {
            self.tiles.insert(tile.coords(), Tile(tile));
        }
    }

    /// Compute a given function for all the neighbours including above and below
    pub fn neighbouring<F, T>(&self, coords: Coords, func: F) -> Neighbouring<T>
    where
        F: Fn(Option<&Tile<'a>>, &Vec<Building<'a>>) -> T,
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
        F: Fn(Option<&Tile<'a>>, &Vec<Building<'a>>) -> T,
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
                let wally = self
                    .tiles
                    .get(&Coords::new(coords.x + x, coords.y + y, z))
                    .some_and(|tile| tile.is_wall());
                if wally {
                    if x == -1 {
                        wallyness[W] += 1;
                    }

                    if x == 1 {
                        wallyness[E] += 1;
                    }

                    if y == -1 {
                        wallyness[N] += 1;
                    }

                    if y == 1 {
                        wallyness[S] += 1;
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
