use crate::{tile::Tile, rfr::DFTile};
use dfhack_remote::Coord;
use std::{collections::HashMap, ops::Add};

/// Intermediary format between DF and voxels
pub struct Map {
    pub tiles: HashMap<Coords, Tile>,
    pub dimensions: [i32; 3],
}
pub enum Direction {
    Above,
    Below,
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Coords {
    pub x: i32,
    pub y: i32,
    pub z: i32,
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
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            tiles: Default::default(),
            dimensions: [x, y, z],
        }
    }
    pub fn add_tile<'a>(&mut self, df_tile: &'a DFTile<'a>) {
        if df_tile.hidden {
            return;
        }

        if let Some(tile) = df_tile.into() {
            let coord_mirrored = Coords::new(df_tile.coords.x, df_tile.coords.y, df_tile.coords.z);
            self.tiles.insert(coord_mirrored, tile);
        }
    }

    pub fn has_tree_at_coords(&self, coords: &Coords, tree_origin: &Coords) -> bool {
        self.tiles
            .get(coords)
            .some_and(|t| t.is_from_tree(tree_origin))
    }
}

impl Direction {
    pub fn get_coords(&self) -> Coords {
        match self {
            Direction::Above => Coords::new(0, 0, 1),
            Direction::Below => Coords::new(0, 0, -1),
            Direction::North => Coords::new(0, -1, 0),
            Direction::South => Coords::new(0, 1, 0),
            Direction::East => Coords::new(1, 0, 0),
            Direction::West => Coords::new(-1, 0, 0),
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

impl Add<Direction> for Coords {
    type Output = Coords;

    fn add(self, rhs: Direction) -> Self::Output {
        self + rhs.get_coords()
    }
}
