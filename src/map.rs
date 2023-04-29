use crate::{
    building::{Building, BuildingType},
    rfr::DFTile,
    tile::Tile,
};
use dfhack_remote::{BuildingInstance, Coord};
use std::{collections::HashMap, ops::Add};

/// Intermediary format between DF and voxels
pub struct Map {
    pub tiles: HashMap<Coords, Tile>,
    pub buildings: HashMap<Coords, Building>,
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
            buildings: Default::default(),
            dimensions: [x, y, z],
        }
    }
    pub fn add_tile<'a>(&mut self, df_tile: &'a DFTile<'a>) {
        if let Some(tile) = df_tile.into() {
            let coords = Coords::new(df_tile.coords.x, df_tile.coords.y, df_tile.coords.z);
            self.tiles.insert(coords, tile);
        }
    }

    pub fn add_building(&mut self, df_building: BuildingInstance) {
        let coords = Coords::new(
            df_building.pos_x_min(),
            df_building.pos_y_min(),
            df_building.pos_z_min(),
        );
        if let Some(building) = Building::from_df_building(df_building) {
            self.buildings.insert(coords, building);
        }
    }

    pub fn has_tree_at_coords(&self, coords: &Coords, tree_origin: &Coords) -> bool {
        self.tiles
            .get(coords)
            .some_and(|t| t.is_from_tree(tree_origin))
    }

    pub fn connect_window_to_coords(&self, coords: Coords) -> bool {
        self.buildings.get(&coords).some_and(|b| {
            b.building_type == BuildingType::WindowGem
                || b.building_type == BuildingType::WindowGlass
        }) || self.tiles.get(&coords).some_and(|t| {
            matches!(
                t.shape,
                crate::tile::Shape::Fortification | crate::tile::Shape::Full
            )
        })
    }

    pub fn connect_door_to_coords(&self, coords: &Coords) -> bool {
        // Connect to wall and doors
        self.buildings
            .get(coords)
            .some_and(|b| b.building_type == BuildingType::Door)
            || self.tiles.get(coords).some_and(|t| {
                matches!(
                    t.shape,
                    crate::tile::Shape::Fortification | crate::tile::Shape::Full
                )
            })
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
