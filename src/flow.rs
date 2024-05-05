use std::ops::Add;

use crate::{
    block::BLOCK_SIZE,
    context::DFContext,
    coords::DFLocalCoords,
    palette::{DefaultMaterials, Material, Palette},
    shape::{self, slice_empty, Box3D},
    voxel::voxels_from_uniform_shape,
    DFMapCoords, StableRng, WithDFCoords,
};
use dfhack_remote::{FlowInfo, FlowType};
use rand::Rng;

#[easy_ext::ext(FlowInfoExt)]
impl FlowInfo {
    pub fn build(&self, context: &DFContext, palette: &mut Palette) -> Vec<dot_vox::Voxel> {
        let coords = self.coords();
        // The flows are stored in the blocks, but their coords are global
        let local_coords = DFLocalCoords {
            x: (coords.x as usize % BLOCK_SIZE) as u8,
            y: (coords.y as usize % BLOCK_SIZE) as u8,
        };
        let mut rng = self.stable_rng();
        let shape: Box3D<bool> = match self.type_() {
            FlowType::OceanWave => [
                slice_empty(),
                slice_empty(),
                slice_empty(),
                shape::slice_from_fn(|_, _| {
                    rng.gen_ratio(self.density().abs().min(100).max(0) as u32, 400)
                }),
                shape::slice_from_fn(|_, _| {
                    rng.gen_ratio(self.density().abs().min(100).max(0) as u32, 400)
                }),
            ],
            _ => shape::box_from_fn(|_, _, _| {
                rng.gen_ratio(self.density().abs().min(100).max(0) as u32, 400)
            }),
        };
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

        voxels_from_uniform_shape(shape, local_coords, palette.get(&material, context))
    }
}

impl WithDFCoords for FlowInfo {
    fn coords(&self) -> DFMapCoords {
        self.pos.get_or_default().into()
    }
}

impl<T> Add<T> for DFMapCoords
where
    T: WithDFCoords,
{
    type Output = DFMapCoords;

    fn add(self, rhs: T) -> Self::Output {
        self + rhs.coords()
    }
}
