use std::{
    fmt::Display,
    ops::{Add, RangeInclusive},
};

pub const BASE: usize = 3;
pub const HEIGHT: usize = 5;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VoxelCoords {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub trait WithVoxelCoords {
    fn coords(&self) -> VoxelCoords;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DFCoords {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub trait WithDFCoords {
    fn coords(&self) -> DFCoords;
}

#[derive(Debug, Clone)]
pub struct DFBoundingBox {
    pub x: RangeInclusive<i32>,
    pub y: RangeInclusive<i32>,
    pub z: RangeInclusive<i32>,
}

impl DFCoords {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl VoxelCoords {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn from_df(origin: DFCoords, sub_x: usize, sub_y: usize, sub_z: usize) -> Self {
        Self::new(
            origin.x * BASE as i32 + sub_x as i32,
            origin.y * BASE as i32 + sub_y as i32,
            origin.z * HEIGHT as i32 + sub_z as i32,
        )
    }

    pub fn from_prefab_voxel(
        origin: DFCoords,
        prefab_model: &dot_vox::Model,
        voxel: &dot_vox::Voxel,
    ) -> Self {
        let max_y = prefab_model.size.y as i32 - 1;
        Self::new(
            origin.x * BASE as i32 + voxel.x as i32,
            origin.y * BASE as i32 + (max_y - voxel.y as i32),
            origin.z * HEIGHT as i32 + voxel.z as i32,
        )
    }
}

impl From<dfhack_remote::Coord> for DFCoords {
    fn from(value: dfhack_remote::Coord) -> Self {
        Self {
            x: value.x(),
            y: value.y(),
            z: value.z(),
        }
    }
}

impl From<&dfhack_remote::Coord> for DFCoords {
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

impl Add<DFCoords> for DFCoords {
    type Output = DFCoords;

    fn add(self, rhs: DFCoords) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<'a> Add<DFCoords> for &'a DFCoords {
    type Output = DFCoords;

    fn add(self, rhs: DFCoords) -> Self::Output {
        DFCoords::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Display for VoxelCoords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}

impl Display for DFCoords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}

impl DFBoundingBox {
    pub fn new(x: RangeInclusive<i32>, y: RangeInclusive<i32>, z: RangeInclusive<i32>) -> Self {
        Self { x, y, z }
    }

    pub fn origin(&self) -> DFCoords {
        DFCoords::new(*self.x.start(), *self.y.start(), *self.z.start())
    }

    pub fn contains(&self, coords: DFCoords) -> bool {
        self.x.contains(&coords.x) && self.y.contains(&coords.y) && self.z.contains(&coords.z)
    }
}
