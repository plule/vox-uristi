use crate::{DFCoords, WithDFCoords};
use dfhack_remote::BuildingDirection;
use std::ops::{BitOr};

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

pub trait Rotating {
    /// Return a copy facing away from a given direction, assuming
    /// the input was looking at south
    fn facing_away(self, direction: DirectionFlat) -> Self
    where
        Self: Sized,
    {
        let n = match direction {
            DirectionFlat::North => 0,
            DirectionFlat::East => 1,
            DirectionFlat::South => 2,
            DirectionFlat::West => 3,
        };
        self.rotated_by(n)
    }

    /// Return a copy facing away from a given direction, assuming
    /// the input was looking at south
    fn looking_at(self, direction: DirectionFlat) -> Self
    where
        Self: Sized,
    {
        let n = match direction {
            DirectionFlat::South => 0,
            DirectionFlat::West => 1,
            DirectionFlat::North => 2,
            DirectionFlat::East => 3,
        };
        self.rotated_by(n)
    }

    /// Return a copy rotated by amount time 90 degrees
    fn rotated_by(self, amount: usize) -> Self;
}

impl BitOr for NeighbouringFlat<bool> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            n: self.n | rhs.n,
            e: self.e | rhs.e,
            s: self.s | rhs.s,
            w: self.w | rhs.w,
        }
    }
}

impl WithDFCoords for Direction8Flat {
    fn coords(&self) -> DFCoords {
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

impl WithDFCoords for Direction {
    fn coords(&self) -> DFCoords {
        match self {
            Direction::Above => DFCoords::new(0, 0, 1),
            Direction::Below => DFCoords::new(0, 0, -1),
            Direction::North => DFCoords::new(0, -1, 0),
            Direction::South => DFCoords::new(0, 1, 0),
            Direction::East => DFCoords::new(1, 0, 0),
            Direction::West => DFCoords::new(-1, 0, 0),
        }
    }
}

impl WithDFCoords for DirectionFlat {
    fn coords(&self) -> DFCoords {
        match self {
            DirectionFlat::North => DFCoords::new(0, -1, 0),
            DirectionFlat::South => DFCoords::new(0, 1, 0),
            DirectionFlat::East => DFCoords::new(1, 0, 0),
            DirectionFlat::West => DFCoords::new(-1, 0, 0),
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

impl NeighbouringFlat<bool> {
    pub fn directions(&self) -> Vec<DirectionFlat> {
        let mut ret = Vec::new();
        if self.n {
            ret.push(DirectionFlat::North);
        }

        if self.e {
            ret.push(DirectionFlat::East);
        }

        if self.s {
            ret.push(DirectionFlat::South);
        }

        if self.w {
            ret.push(DirectionFlat::West);
        }

        ret
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
