use super::{BuildingInstanceExt, FurnaceType, WorkshopType};
use crate::{
    palette::Material,
    shape::{self, Box3D},
    voxel::{voxels_from_uniform_shape, FromDotVox, Voxel, WithDotVoxMaterials},
};
use dfhack_remote::BuildingInstance;
use easy_ext::ext;
use itertools::Itertools;

impl WithDotVoxMaterials for BuildingInstance {
    fn dot_vox_materials(&self) -> Vec<Material> {
        self.items
            .iter()
            .map(|item| Material::Generic(item.item.material.get_or_default().clone()))
            .collect_vec()
    }
}

#[ext(BuildingInstanceWorkshopExt)]
pub impl BuildingInstance {
    fn collect_workshop_voxels(&self, workshop_type: WorkshopType) -> Vec<Voxel> {
        match workshop_type {
            _ => {
                let dimensions = self.dimension();
                if dimensions.0 == 3 && dimensions.1 == 3 {
                    self.dot_vox(include_bytes!("workshop.vox"))
                } else {
                    let shape: Box3D<bool> = [
                        shape::slice_empty(),
                        shape::slice_empty(),
                        shape::slice_empty(),
                        shape::slice_full(),
                        shape::slice_full(),
                    ];
                    voxels_from_uniform_shape(shape, self.origin(), self.material())
                }
            }
        }
    }

    fn collect_furnace_voxels(&self, furnace_type: FurnaceType) -> Vec<Voxel> {
        match furnace_type {
            FurnaceType::Generic
            | FurnaceType::WoodFurnace
            | FurnaceType::GlassFurnace
            | FurnaceType::MagmaGlassFurnace
            | FurnaceType::MagmaKiln
            | FurnaceType::Kiln
            | FurnaceType::Custom => self.dot_vox(include_bytes!("furnace.vox")),
            FurnaceType::Smelter | FurnaceType::MagmaSmelter => {
                self.dot_vox(include_bytes!("smelter.vox"))
            }
        }
    }
}
