//! Voxel generation tooling
use crate::{coords::DFLocalCoords, direction::Rotating, export::BLOCK_SIZE, shape::Box3D, BASE};
use itertools::Itertools;

pub fn voxels_from_shape<const B: usize, const H: usize>(
    shape: Box3D<Option<u8>, B, H>,
    origin: DFLocalCoords,
) -> Vec<dot_vox::Voxel> {
    (0..B)
        .cartesian_product(0..B)
        .cartesian_product(0..H)
        .filter_map(|((x, y), z)| {
            shape[H - 1 - z][y][x].as_ref().map(|material| {
                let x = origin.x * BASE as u8 + x as u8;
                let y = (BLOCK_SIZE as u8 - origin.y - 1) * BASE as u8 + (B - y - 1) as u8;
                let z = z as u8;
                dot_vox::Voxel {
                    x,
                    y,
                    z,
                    i: *material,
                }
            })
        })
        .collect()
}

pub fn voxels_from_uniform_shape<const B: usize, const H: usize>(
    shape: Box3D<bool, B, H>,
    origin: DFLocalCoords,
    material: u8,
) -> Vec<dot_vox::Voxel> {
    let shape = shape.map(|slice| {
        slice.map(|col| col.map(|include| if include { Some(material) } else { None }))
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
