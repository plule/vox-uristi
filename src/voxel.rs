use dfhack_remote::PlantRawList;

use crate::{
    export::ExportSettings,
    map::{Coords, Map},
    palette::Material,
    shape::Box3D,
};

#[derive(Debug)]
pub struct Voxel {
    pub coord: Coords,
    pub material: Material,
}

impl Voxel {
    pub fn new(coord: Coords, material: Material) -> Self {
        Self { coord, material }
    }
}

pub trait CollectVoxels {
    fn collect_voxels(
        &self,
        map: &Map,
        settings: &ExportSettings,
        plant_raws: &PlantRawList,
    ) -> Vec<Voxel>;
}

pub fn voxels_from_shape<const B: usize, const H: usize>(
    shape: Box3D<Option<Material>, B, H>,
    origin: Coords,
) -> Vec<Voxel> {
    let mut ret = Vec::new();
    for x in 0..B {
        for y in 0..B {
            for z in 0..H {
                let coords = Coords {
                    x: origin.x * 3 + x as i32,
                    y: origin.y * 3 + y as i32,
                    z: origin.z * 5 + z as i32,
                };
                if let Some(material) = &shape[H - 1 - z][y][x] {
                    ret.push(Voxel::new(coords, material.clone()))
                }
            }
        }
    }
    ret
}

pub fn voxels_from_uniform_shape<const B: usize, const H: usize>(
    shape: Box3D<bool, B, H>,
    origin: Coords,
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
