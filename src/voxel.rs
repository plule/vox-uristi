use crate::{
    building::{BoundingBox, BuildingInstanceExt},
    context::DFContext,
    direction::{DirectionFlat, NeighbouringFlat, Rotating},
    map::Map,
    palette::{DefaultMaterials, Material},
    prefabs::{Connectivity, ContentMode, OrientationMode, Prefab},
    shape::Box3D,
    tile::BlockTileExt,
    Coords, IsSomeAnd,
};
use dfhack_remote::MatPair;
use dot_vox::Model;
use itertools::Itertools;
use std::iter::repeat;

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
    fn collect_voxels(&self, map: &Map, context: &DFContext) -> Vec<Voxel>;
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
    fn build_materials(&self) -> Box<dyn Iterator<Item = MatPair> + '_>;
    fn content_materials(&self) -> Box<dyn Iterator<Item = MatPair> + '_>;
    fn df_orientation(&self) -> Option<DirectionFlat>;
    fn bounding_box(&self) -> BoundingBox;
    fn self_connectivity(&self, map: &Map, context: &DFContext) -> NeighbouringFlat<bool>;

    fn voxels_from_prefab(&self, prefab: &Prefab, map: &Map, context: &DFContext) -> Vec<Voxel> {
        let mut model = Model {
            size: prefab.model.size,
            voxels: prefab.model.voxels.clone(),
        };

        let bounding_box = self.bounding_box();
        let coords = bounding_box.origin();

        // Rotate the model based on the preference
        match prefab.orientation {
            OrientationMode::FromDwarfFortress => {
                if let Some(direction) = self.df_orientation() {
                    model = model.looking_at(direction);
                }
            }
            OrientationMode::AgainstWall => {
                model = model.facing_away(map.wall_direction(coords));
            }
            OrientationMode::FacingChairOrAgainstWall => {
                let c = map.neighbouring_flat(coords, |_, n| n.iter().any(|b| b.is_chair(context)));
                if let Some(chair_direction) = c.directions().first() {
                    model = model.looking_at(*chair_direction)
                } else {
                    model = model.facing_away(map.wall_direction(coords));
                }
            }
        }

        // Collect the material palette
        // First 8 materials of the palette are the build materials
        let build_materials = self
            .build_materials()
            .map(|m| Some(Material::Generic(m)))
            .chain(repeat(None))
            .take(8);
        // Next 8 materials are the darker versions
        let dark_build_materials = self
            .build_materials()
            .map(|m| Some(Material::DarkGeneric(m)))
            .chain(repeat(None))
            .take(8);
        // Next 8 are the content materials
        let content_materials = match prefab.content {
            ContentMode::Unique => self
                .content_materials()
                .unique_by(|m| (m.mat_index(), m.mat_type()))
                .take(8)
                .collect_vec(),
            ContentMode::All => self.content_materials().take(8).collect_vec(),
        }
        .into_iter()
        .map(|m| Some(Material::Generic(m)))
        .chain(repeat(None))
        .take(8);
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

        // Apply connectivity rules
        let mut model_voxels = model.voxels.clone();
        match prefab.connectivity {
            Connectivity::None => {}
            Connectivity::SelfOrWall => {
                let c1 =
                    map.neighbouring_flat(coords, |tile, _| tile.some_and(|tile| tile.is_wall()));
                let c2 = self.self_connectivity(map, context);
                let cx = (model.size.x / 2) as i32;
                let cy = (model.size.y / 2) as i32;
                model_voxels.retain(|voxel| {
                    let mut display = true;
                    let x = voxel.x as i32 - cx;
                    let y = voxel.y as i32 - cy;
                    if x < 0 {
                        display &= c1.w || c2.w;
                    }
                    if x > 0 {
                        display &= c1.e || c2.e;
                    }
                    if y < 0 {
                        display &= c1.s || c2.s;
                    }
                    if y > 0 {
                        display &= c1.n || c2.n;
                    }
                    display
                });
            }
        }

        // Convert to voxels with materials, positionned globally
        let mut voxels = Vec::new();
        let max_y = model.size.y as i32 - 1;
        for x in bounding_box.x.clone().step_by(model.size.x as usize / 3) {
            for y in bounding_box.y.clone().step_by(model.size.y as usize / 3) {
                for z in bounding_box.z.clone() {
                    let coords = Coords::new(x, y, z);

                    voxels.extend(model_voxels.iter().filter_map(|voxel| {
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
