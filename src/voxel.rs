use crate::{
    building::BuildingInstanceExt,
    context::DFContext,
    direction::{DirectionFlat, NeighbouringFlat, Rotating},
    map::Map,
    palette::{DefaultMaterials, Material},
    prefabs::{Connectivity, ContentMode, OrientationMode, Prefab},
    shape::Box3D,
    tile::BlockTileExt,
    DFBoundingBox, DFCoords, IsSomeAnd, VoxelCoords, BASE,
};
use dfhack_remote::MatPair;
use dot_vox::Model;
use itertools::Itertools;
use std::iter::repeat;

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

pub trait CollectVoxels {
    fn collect_voxels(&self, map: &Map, context: &DFContext) -> Vec<Voxel>;
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

pub trait FromPrefab {
    fn build_materials(&self) -> Box<dyn Iterator<Item = MatPair> + '_>;
    fn content_materials(&self) -> Box<dyn Iterator<Item = MatPair> + '_>;
    fn df_orientation(&self) -> Option<DirectionFlat>;
    fn bounding_box(&self) -> DFBoundingBox;
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
                let c = map
                    .neighbouring_flat(coords, |n| n.buildings.iter().any(|b| b.is_chair(context)));
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
            Some(Material::Default(DefaultMaterials::Light)),
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
                let wall_connectivity = map.neighbouring_flat(coords, |tile| {
                    tile.block_tile.some_and(|tile| tile.is_wall())
                });
                let neighbour_connectivity = self.self_connectivity(map, context);
                let c = wall_connectivity | neighbour_connectivity;
                let cx = (model.size.x / 2) as i32;
                let cy = (model.size.y / 2) as i32;
                model_voxels.retain(|voxel| {
                    let mut display = true;
                    let x = voxel.x as i32 - cx;
                    let y = voxel.y as i32 - cy;
                    if x < 0 {
                        display &= c.w;
                    }
                    if x > 0 {
                        display &= c.e;
                    }
                    if y < 0 {
                        display &= c.s;
                    }
                    if y > 0 {
                        display &= c.n
                    }
                    display
                });
            }
            Connectivity::SelfRemovesLayer(layer) => {
                let neighbour_connectivity = self.self_connectivity(map, context);
                let self_connectivity =
                    NeighbouringFlat::new(|dir| bounding_box.contains(coords + dir));
                let c = neighbour_connectivity | self_connectivity;
                let cx = (model.size.x / 2) as i32;
                let cy = (model.size.y / 2) as i32;
                model_voxels.retain(|voxel| {
                    let mut display = true;
                    let x = voxel.x as i32 - cx;
                    let y = voxel.y as i32 - cy;
                    let z = voxel.z;
                    if x < 0 && z == layer {
                        display &= !c.w;
                    }
                    if x > 0 && z == layer {
                        display &= !c.e;
                    }
                    if y < 0 && z == layer {
                        display &= !c.s;
                    }
                    if y > 0 && z == layer {
                        display &= !c.n;
                    }
                    display
                });
            }
        }

        // Convert to voxels with materials, positionned globally
        let mut voxels = Vec::new();
        for x in bounding_box.x.clone().step_by(model.size.x as usize / BASE) {
            for y in bounding_box.y.clone().step_by(model.size.y as usize / BASE) {
                for z in bounding_box.z.clone() {
                    let coords = DFCoords::new(x, y, z);

                    voxels.extend(model_voxels.iter().filter_map(|voxel| {
                        let material = materials.get(voxel.i as usize).cloned().flatten();

                        material.map(|material| {
                            Voxel::new(
                                VoxelCoords::from_prefab_voxel(coords, &model, voxel),
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
