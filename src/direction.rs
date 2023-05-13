use crate::{Coords, WithCoords};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction8Flat {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

#[derive(Debug, PartialEq)]
pub struct NeighbouringFlat<T> {
    pub n: T,
    pub e: T,
    pub s: T,
    pub w: T,
}

#[derive(Debug, PartialEq)]
pub struct Neighbouring8Flat<T> {
    pub n: T,
    pub ne: T,
    pub e: T,
    pub se: T,
    pub s: T,
    pub sw: T,
    pub w: T,
    pub nw: T,
}

pub struct Neighbouring<T> {
    pub a: T,
    pub b: T,
    pub n: T,
    pub e: T,
    pub s: T,
    pub w: T,
}

impl WithCoords for Direction8Flat {
    fn coords(&self) -> Coords {
        match self {
            Direction8Flat::North => Direction::North.coords(),
            Direction8Flat::NorthEast => Direction::North.coords() + Direction::East.coords(),
            Direction8Flat::East => Direction::East.coords(),
            Direction8Flat::SouthEast => Direction::South.coords() + Direction::East.coords(),
            Direction8Flat::South => Direction::South.coords(),
            Direction8Flat::SouthWest => Direction::South.coords() + Direction::West.coords(),
            Direction8Flat::West => Direction::West.coords(),
            Direction8Flat::NorthWest => Direction::North.coords() + Direction::West.coords(),
        }
    }
}

impl WithCoords for Direction {
    fn coords(&self) -> Coords {
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

impl WithCoords for DirectionFlat {
    fn coords(&self) -> Coords {
        match self {
            DirectionFlat::North => Coords::new(0, -1, 0),
            DirectionFlat::South => Coords::new(0, 1, 0),
            DirectionFlat::East => Coords::new(1, 0, 0),
            DirectionFlat::West => Coords::new(-1, 0, 0),
        }
    }
}

impl DirectionFlat {
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

impl<T> Neighbouring8Flat<T> {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(Direction8Flat) -> T,
    {
        Self {
            n: func(Direction8Flat::North),
            ne: func(Direction8Flat::NorthEast),
            e: func(Direction8Flat::East),
            se: func(Direction8Flat::SouthEast),
            s: func(Direction8Flat::South),
            sw: func(Direction8Flat::SouthWest),
            w: func(Direction8Flat::West),
            nw: func(Direction8Flat::NorthWest),
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

impl<T> Add<T> for Coords
where
    T: WithCoords,
{
    type Output = Coords;

    fn add(self, rhs: T) -> Self::Output {
        self + rhs.coords()
    }
}
