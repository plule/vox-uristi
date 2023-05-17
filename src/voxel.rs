use std::collections::HashMap;

use dfhack_remote::{BuildingDefinition, PlantRawList};
use dot_vox::Model;

use crate::{
    direction::Rotating,
    export::ExportSettings,
    map::Map,
    palette::{DefaultMaterials, Material},
    shape::Box3D,
    Coords, WithCoords,
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
        building_defs: &HashMap<(i32, i32, i32), BuildingDefinition>,
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

pub fn voxels_from_dot_vox(model: &Model, origin: Coords, materials: &[Material]) -> Vec<Voxel> {
    let max_y = model.size.y as i32 - 1;
    model
        .voxels
        .iter()
        .filter_map(|voxel| {
            let material = match voxel.i {
                i if i < 8 => materials.get(i as usize).cloned(),
                8 => Some(Material::Default(DefaultMaterials::Fire)),
                9 => Some(Material::Default(DefaultMaterials::Wood)),
                _ => None,
            };

            material.map(|material| {
                Voxel::new(
                    Coords::new(
                        voxel.x as i32 + origin.x * 3,
                        (max_y - voxel.y as i32) + origin.y * 3,
                        voxel.z as i32 + origin.z * 5,
                    ),
                    material,
                )
            })
        })
        .collect()
}

pub trait FromDotVox {
    fn dot_vox(&self, voxels: &[u8]) -> Vec<Voxel>;
}

pub trait WithDotVoxMaterials {
    fn dot_vox_materials(&self) -> Vec<Material>;
}

impl<T> FromDotVox for T
where
    T: WithCoords + WithDotVoxMaterials,
{
    fn dot_vox(&self, bytes: &[u8]) -> Vec<Voxel> {
        voxels_from_dot_vox(
            &dot_vox::load_bytes(bytes).expect("Invalid model").models[0],
            self.coords(),
            &self.dot_vox_materials(),
        )
    }
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
