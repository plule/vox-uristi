use crate::map::Coords;
use dfhack_remote::BuildingDirection;
use std::ops::Add;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Above,
    Below,
    North,
    East,
    South,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionFlat {
    North,
    East,
    South,
    West,
}

#[derive(Debug, PartialEq)]
pub struct NeighbouringFlat<T> {
    pub n: T,
    pub e: T,
    pub s: T,
    pub w: T,
}

pub struct Neighbouring<T> {
    pub a: T,
    pub b: T,
    pub n: T,
    pub e: T,
    pub s: T,
    pub w: T,
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

impl<T> NeighbouringFlat<T> {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(DirectionFlat) -> T,
    {
        Self {
            n: func(DirectionFlat::North),
            e: func(DirectionFlat::East),
            s: func(DirectionFlat::South),
            w: func(DirectionFlat::West),
        }
    }
}

impl<T> Neighbouring<T> {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(Direction) -> T,
    {
        Self {
            a: func(Direction::Above),
            b: func(Direction::Below),
            n: func(Direction::North),
            e: func(Direction::East),
            s: func(Direction::South),
            w: func(Direction::West),
        }
    }
}
