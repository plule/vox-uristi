use crate::map::Coords;
use dfhack_remote::BuildingDirection;
use std::ops::Add;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Above,
    Below,
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionFlat {
    North,
    South,
    East,
    West,
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

impl Add<Direction> for Coords {
    type Output = Coords;

    fn add(self, rhs: Direction) -> Self::Output {
        self + rhs.get_coords()
    }
}

impl DirectionFlat {
    pub fn get_coords(&self) -> Coords {
        match self {
            DirectionFlat::North => Coords::new(0, -1, 0),
            DirectionFlat::South => Coords::new(0, 1, 0),
            DirectionFlat::East => Coords::new(1, 0, 0),
            DirectionFlat::West => Coords::new(-1, 0, 0),
        }
    }

    pub fn maybe_from_df(value: &BuildingDirection) -> Option<Self> {
        match value {
            BuildingDirection::NORTH => Some(DirectionFlat::North),
            BuildingDirection::EAST => Some(DirectionFlat::East),
            BuildingDirection::SOUTH => Some(DirectionFlat::South),
            BuildingDirection::WEST => Some(DirectionFlat::West),
            BuildingDirection::NONE => None,
        }
    }
}

impl Add<DirectionFlat> for Coords {
    type Output = Coords;

    fn add(self, rhs: DirectionFlat) -> Self::Output {
        self + rhs.get_coords()
    }
}
