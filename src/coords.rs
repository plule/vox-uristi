use std::{
    collections::hash_map::DefaultHasher,
    fmt::Display,
    hash::{Hash, Hasher},
    ops::{Add, RangeInclusive, Sub},
};

use rand::{rngs::StdRng, SeedableRng};

use crate::{block::BLOCK_SIZE, StableRng};

pub const BASE: usize = 3;
pub const HEIGHT: usize = 5;

/// Global voxel coordinates
/// They are in voxel (multiplied by BASE and HEIGHT), but
/// they are oriented like in dwarf fortress and not fitted to zero
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VoxelCoords {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Coordinates in the global dot_vox space
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub struct DotVoxModelCoords {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl DotVoxModelCoords {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

pub trait WithBoundingBox {
    fn bounding_box(&self) -> DFBoundingBox;
}

/// Coordinates of a tile in the dwarf fortress map
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DFMapCoords {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Coordinates of a tile in a dwarf fortress block
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DFLocalCoords {
    pub x: u8,
    pub y: u8,
}

impl DFLocalCoords {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    pub fn from_index(index: usize) -> Self {
        Self {
            x: (index % BLOCK_SIZE) as u8,
            y: (index / BLOCK_SIZE) as u8,
        }
    }
}

/// Coordinates of a block in the dwarf fortress map
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DFBlockCoords {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl DFBlockCoords {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl Add<DFLocalCoords> for DFBlockCoords {
    type Output = DFMapCoords;

    fn add(self, rhs: DFLocalCoords) -> Self::Output {
        DFMapCoords::new(self.x + rhs.x as i32, self.y + rhs.y as i32, self.z)
    }
}

pub struct DFDimensions {
    pub x: u32,
    pub y: u32,
}

pub trait WithDFCoords {
    fn coords(&self) -> DFMapCoords;
}

pub trait WithBlockCoords {
    fn block_coords(&self) -> DFBlockCoords;
}

#[derive(Debug, Clone)]
pub struct DFBoundingBox {
    pub x: RangeInclusive<i32>,
    pub y: RangeInclusive<i32>,
    pub z: RangeInclusive<i32>,
}

impl DFMapCoords {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl VoxelCoords {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn from_local_df(origin: DFLocalCoords, sub_x: usize, sub_y: usize, sub_z: usize) -> Self {
        Self::new(
            origin.x as i32 * BASE as i32 + sub_x as i32,
            BLOCK_SIZE as i32 - origin.y as i32 * BASE as i32 + sub_y as i32,
            sub_z as i32,
        )
    }

    pub fn from_df(origin: DFMapCoords, sub_x: usize, sub_y: usize, sub_z: usize) -> Self {
        Self::new(
            origin.x * BASE as i32 + sub_x as i32,
            origin.y * BASE as i32 + sub_y as i32,
            origin.z * HEIGHT as i32 + sub_z as i32,
        )
    }

    pub fn into_global_coords(self, max_x: i32, max_y: i32, min_z: i32) -> DotVoxModelCoords {
        DotVoxModelCoords {
            x: self.x - max_x,
            y: max_y - self.y,
            z: self.z - min_z,
        }
    }

    pub fn into_level_global_coords(self, max_x: i32, max_y: i32) -> DotVoxModelCoords {
        DotVoxModelCoords {
            x: self.x - max_x,
            y: max_y - self.y,
            z: 0,
        }
    }
}

impl From<DFDimensions> for dot_vox::Size {
    fn from(value: DFDimensions) -> Self {
        Self {
            x: value.x * BASE as u32,
            y: value.y * BASE as u32,
            z: HEIGHT as u32,
        }
    }
}

impl From<dot_vox::Size> for DFDimensions {
    fn from(value: dot_vox::Size) -> Self {
        Self {
            x: value.x / BASE as u32,
            y: value.y / BASE as u32,
        }
    }
}

impl From<dfhack_remote::Coord> for DFMapCoords {
    fn from(value: dfhack_remote::Coord) -> Self {
        Self {
            x: value.x(),
            y: value.y(),
            z: value.z(),
        }
    }
}

impl From<&dfhack_remote::Coord> for DFMapCoords {
    fn from(value: &dfhack_remote::Coord) -> Self {
        Self {
            x: value.x(),
            y: value.y(),
            z: value.z(),
        }
    }
}

impl Add<VoxelCoords> for VoxelCoords {
    type Output = VoxelCoords;

    fn add(self, rhs: VoxelCoords) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<'a> Add<VoxelCoords> for &'a VoxelCoords {
    type Output = VoxelCoords;

    fn add(self, rhs: VoxelCoords) -> Self::Output {
        VoxelCoords::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Add<DFMapCoords> for DFMapCoords {
    type Output = DFMapCoords;

    fn add(self, rhs: DFMapCoords) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<'a> Add<DFMapCoords> for &'a DFMapCoords {
    type Output = DFMapCoords;

    fn add(self, rhs: DFMapCoords) -> Self::Output {
        DFMapCoords::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Display for VoxelCoords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}

impl Display for DFMapCoords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}

impl DFBoundingBox {
    pub fn new(x: RangeInclusive<i32>, y: RangeInclusive<i32>, z: RangeInclusive<i32>) -> Self {
        Self { x, y, z }
    }

    pub fn origin(&self) -> DFMapCoords {
        DFMapCoords::new(*self.x.start(), *self.y.start(), *self.z.start())
    }

    pub fn contains(&self, coords: DFMapCoords) -> bool {
        self.x.contains(&coords.x) && self.y.contains(&coords.y) && self.z.contains(&coords.z)
    }

    pub fn dimension(&self) -> DFDimensions {
        DFDimensions {
            x: (1 + self.x.end() - self.x.start()) as u32,
            y: (1 + self.y.end() - self.y.start()) as u32,
        }
    }

    pub fn dot_vox_coords(&self) -> VoxelCoords {
        let size = dot_vox::Size::from(self.dimension());
        VoxelCoords::from_df(
            self.origin(),
            // Weird centering due to model coordinates beeing centered
            (size.x as usize) / 2,
            (size.y as usize - 1) / 2,
            2,
        )
    }

    pub fn level_dot_vox_coords(&self) -> VoxelCoords {
        let size = dot_vox::Size::from(self.dimension());
        VoxelCoords::from_df(
            self.origin(),
            // Weird centering due to model coordinates beeing centered
            (size.x as usize) / 2,
            (size.y as usize - 1) / 2,
            0,
        )
    }
}

impl Sub<VoxelCoords> for DFBoundingBox {
    type Output = DFBoundingBox;

    fn sub(self, rhs: VoxelCoords) -> Self::Output {
        Self::new(
            (self.x.start() - rhs.x)..=(self.x.end() - rhs.x),
            (self.y.start() - rhs.y)..=(self.y.end() - rhs.y),
            (self.z.start() - rhs.z)..=(self.z.end() - rhs.z),
        )
    }
}

impl<T> StableRng for T
where
    T: WithDFCoords,
{
    fn stable_rng(&self) -> StdRng {
        let mut s = DefaultHasher::new();
        self.coords().hash(&mut s);
        let hash = s.finish();
        SeedableRng::seed_from_u64(hash)
    }
}
