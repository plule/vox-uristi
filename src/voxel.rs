use crate::{
    building::BoundingBox,
    direction::{DirectionFlat, Rotating},
    export::ExportSettings,
    map::Map,
    palette::{DefaultMaterials, Material},
    prefabs::{OrientationMode, Prefab},
    shape::Box3D,
    Coords,
};
use dfhack_remote::{BuildingDefinition, MatPair, PlantRawList};
use dot_vox::Model;
use std::collections::HashMap;

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

pub trait FromPrefab {
    fn build_materials(&self) -> [Option<MatPair>; 8];
    fn content_materials(&self) -> [Option<MatPair>; 8];
    fn df_orientation(&self) -> Option<DirectionFlat>;
    fn bounding_box(&self) -> BoundingBox;

    fn voxels_from_prefab(&self, prefab: &Prefab, map: &Map) -> Vec<Voxel> {
        let mut model = Model {
            size: prefab.model.size,
            voxels: prefab.model.voxels.clone(),
        };

        let bounding_box = self.bounding_box();
        let coords = bounding_box.origin();

        // Rotate the model based on the preference
        match prefab.orientation_mode {
            OrientationMode::FromDwarfFortress => {
                if let Some(direction) = self.df_orientation() {
                    model = model.looking_at(direction);
                }
            }
            OrientationMode::AgainstWall => {
                model = model.facing_away(map.wall_direction(coords));
            }
        }

        // Collect the material palette
        // First 8 materials of the palette are the build materials
        let build_materials = self
            .build_materials()
            .into_iter()
            .map(|m| m.map(Material::Generic));
        // Next 8 materials are the darker versions
        let dark_build_materials = self
            .build_materials()
            .into_iter()
            .map(|m| m.map(Material::DarkGeneric));
        // Next 8 are the content materials
        let content_materials = self
            .content_materials()
            .into_iter()
            .map(|m| m.map(Material::Generic));
        // Next are the default hard-coded materials
        let default_materials = [
            Some(Material::Default(DefaultMaterials::Fire)),
            Some(Material::Default(DefaultMaterials::Wood)),
        ];

        let materials: Vec<Option<Material>> = build_materials
            .chain(dark_build_materials)
            .chain(content_materials)
            .chain(default_materials)
            .collect();

        // Convert to actual voxels
        let mut voxels = Vec::new();
        let max_y = model.size.y as i32 - 1;
        for x in bounding_box.x.clone().step_by(model.size.x as usize / 3) {
            for y in bounding_box.y.clone().step_by(model.size.y as usize / 3) {
                for z in bounding_box.z.clone() {
                    let coords = Coords::new(x, y, z);

                    voxels.extend(model.voxels.iter().filter_map(|voxel| {
                        let material = materials.get(voxel.i as usize).cloned().flatten();

                        material.map(|material| {
                            Voxel::new(
                                Coords::new(
                                    voxel.x as i32 + coords.x * 3,
                                    (max_y - voxel.y as i32) + coords.y * 3,
                                    voxel.z as i32 + coords.z * 5,
                                ),
                                material,
                            )
                        })
                    }));
                }
            }
        }

        voxels
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
