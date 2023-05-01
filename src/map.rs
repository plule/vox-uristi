use crate::{
    building::Building,
    direction::{DirectionFlat, Neighbouring, NeighbouringFlat},
    rfr::BlockTile,
    tile::{Shape, Tile},
};
use dfhack_remote::{BuildingInstance, Coord};
use itertools::Itertools;
use std::{collections::HashMap, fmt::Display, ops::Add};

/// Intermediary format between DF and voxels
pub struct Map {
    pub tiles: HashMap<Coords, Tile>,
    pub buildings: HashMap<Coords, Vec<Building>>,
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

impl Map {
    pub fn new() -> Self {
        Self {
            tiles: Default::default(),
            buildings: Default::default(),
        }
    }
    pub fn add_tile<'a>(&mut self, df_tile: &'a BlockTile<'a>) {
        if let Some(tile) = df_tile.into() {
            self.tiles.insert(df_tile.coords(), tile);
        }
    }

    pub fn add_building(&mut self, df_building: BuildingInstance) {
        let coords = Coords::new(
            df_building.pos_x_min(),
            df_building.pos_y_min(),
            df_building.pos_z_min(),
        );
        if let Some(building) = Building::from_df_building(df_building) {
            self.buildings.entry(coords).or_default().push(building);
        }
    }

    /// Compute a given function for all the neighbours including above and below
    pub fn neighbouring<F, T>(&self, coords: Coords, func: F) -> Neighbouring<T>
    where
        F: Fn(Option<&Tile>, &Vec<Building>) -> T,
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
        F: Fn(Option<&Tile>, &Vec<Building>) -> T,
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
                    .some_and(|tile| matches!(tile.shape, Shape::Fortification | Shape::Full));
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
