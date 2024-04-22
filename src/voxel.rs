use crate::{
    context::DFContext,
    direction::Rotating,
    map::Map,
    palette::{Material, Palette},
    shape::Box3D,
    DFCoords, VoxelCoords,
};
use dot_vox::Model;
use itertools::Itertools;

#[derive(Debug)]
pub struct Voxel {
    pub coord: VoxelCoords,
    pub material: Material,
}

impl Voxel {
    pub fn new(coord: VoxelCoords, material: Material) -> Self {
        Self { coord, material }
    }
}

/// A dwarf fortress object represented as voxels
pub struct VoxelObject {
    pub model: Model,
    pub name: Option<String>,
    pub layer: u32,
}

pub trait CollectObjectVoxels {
    fn build(&self, map: &Map, context: &DFContext, palette: &mut Palette) -> Option<VoxelObject>;
}

pub trait CollectTerrainVoxels {
    fn collect_terrain_voxels(&self, map: &Map, context: &DFContext) -> Vec<Voxel>;
}

pub fn voxels_from_shape<const B: usize, const H: usize>(
    shape: Box3D<Option<Material>, B, H>,
    origin: DFCoords,
) -> Vec<Voxel> {
    (0..B)
        .cartesian_product(0..B)
        .cartesian_product(0..H)
        .filter_map(|((x, y), z)| {
            shape[H - 1 - z][y][x].as_ref().map(|material| {
                let coords = VoxelCoords::from_df(origin, x, y, z);
                Voxel::new(coords, material.clone())
            })
        })
        .collect()
}

pub fn voxels_from_uniform_shape<const B: usize, const H: usize>(
    shape: Box3D<bool, B, H>,
    origin: DFCoords,
    material: Material,
) -> Vec<Voxel> {
    let shape = shape.map(|slice| {
        slice.map(|col| {
            col.map(|include| {
                if include {
                    Some(material.clone())
                } else {
                    None
                }
            })
        })
    });
    voxels_from_shape(shape, origin)
}

impl Rotating for dot_vox::Model {
    fn rotated_by(mut self, amount: usize) -> Self {
        let amount = amount % 4;

        for _ in 0..amount {
            for voxel in &mut self.voxels {
                (voxel.x, voxel.y) = (voxel.y, (self.size.x as u8 - 1) - voxel.x);
            }

            (self.size.x, self.size.y) = (self.size.y, self.size.x)
        }
        self
    }
}
