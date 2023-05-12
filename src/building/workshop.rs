use super::{BuildingInstanceExt, WorkshopType};
use crate::{
    palette::Material,
    shape::{self, Box3D},
    voxel::{voxels_from_uniform_shape, FromDotVox, Voxel, WithDotVoxMaterials},
};
use dfhack_remote::BuildingInstance;
use extend::ext;
use itertools::Itertools;

impl WithDotVoxMaterials for BuildingInstance {
    fn dot_vox_materials(&self) -> Vec<Material> {
        self.items
            .iter()
            .map(|item| Material::Generic(item.item.material.get_or_default().clone()))
            .collect_vec()
    }
}

#[ext(name = BuildingInstanceWorkshopExt)]
pub impl BuildingInstance {
    fn collect_workshop_voxels(&self, workshop_type: WorkshopType) -> Vec<Voxel> {
        match workshop_type {
            WorkshopType::Ashery => self.from_dot_vox(include_bytes!("workshop_ashery.vox")),
            WorkshopType::Clothiers => self.from_dot_vox(include_bytes!("workshop_clothier.vox")),
            WorkshopType::Farmers => self.from_dot_vox(include_bytes!("workshop_farmer.vox")),
            WorkshopType::Fishery => self.from_dot_vox(include_bytes!("workshop_fishery.vox")),
            WorkshopType::Kitchen => self.from_dot_vox(include_bytes!("workshop_kitchen.vox")),
            WorkshopType::Leatherworks => self.from_dot_vox(include_bytes!("workshop_leather.vox")),
            WorkshopType::MetalsmithsForge => {
                self.from_dot_vox(include_bytes!("workshop_metalsmith.vox"))
            }
            WorkshopType::Loom => self.from_dot_vox(include_bytes!("workshop_loom.vox")),
            WorkshopType::Still => self.from_dot_vox(include_bytes!("workshop_still.vox")),
            _ => {
                let dimensions = self.dimension();
                if dimensions.0 == 3 && dimensions.1 == 3 {
                    self.from_dot_vox(include_bytes!("workshop.vox"))
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
}
