use crate::{
    export::ExportSettings,
    palette::{DefaultMaterials, Material},
    shape::{self, Box3D},
    voxel::{voxels_from_uniform_shape, CollectVoxels, Voxel},
    Coords,
};
use dfhack_remote::{FlowInfo, FlowType, PlantRawList};
use rand::Rng;

impl CollectVoxels for &FlowInfo {
    fn collect_voxels(
        &self,
        _map: &crate::map::Map,
        _settings: &ExportSettings,
        _plant_raws: &PlantRawList,
    ) -> Vec<Voxel> {
        let coords = self.coords();
        let shape: Box3D<bool> = shape::box_from_fn(|_, _, _| {
            rand::thread_rng().gen_ratio(self.density().abs().min(100).max(0) as u32, 200)
        });
        let material = match self.type_() {
            FlowType::Mist | FlowType::SeaFoam | FlowType::Steam => {
                Material::Default(DefaultMaterials::Mist)
            }
            FlowType::OceanWave => Material::Default(DefaultMaterials::Water),
            FlowType::MagmaMist => Material::Default(DefaultMaterials::Magma),
            FlowType::Fire | FlowType::CampFire | FlowType::Dragonfire => {
                Material::Default(DefaultMaterials::Fire)
            }
            FlowType::Miasma => Material::Default(DefaultMaterials::Miasma),
            FlowType::Smoke => Material::Default(DefaultMaterials::Smoke),
            FlowType::ItemCloud
            | FlowType::MaterialDust
            | FlowType::MaterialGas
            | FlowType::MaterialVapor
            | FlowType::Web => Material::Generic(self.material.get_or_default().to_owned()),
        };

        voxels_from_uniform_shape(shape, coords, material)
    }
}

pub trait FlowExtensions {
    fn coords(&self) -> Coords;
}

impl FlowExtensions for FlowInfo {
    fn coords(&self) -> Coords {
        self.pos.get_or_default().into()
    }
}
