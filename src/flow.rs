use crate::{
    map::Coords,
    palette::{DefaultMaterials, Material},
    shape::{self, Box3D},
    voxel::{CollectVoxels, Voxel},
};
use dfhack_remote::{FlowInfo, FlowType};
use itertools::Itertools;
use rand::Rng;

pub struct Flow {
    pub info: FlowInfo,
    pub material: Material,
}

impl Flow {
    pub fn new(info: FlowInfo) -> Self {
        let material = match info.type_() {
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
            | FlowType::Web => Material::Generic(info.material.get_or_default().to_owned()),
        };
        Self { info, material }
    }

    pub fn coords(&self) -> Coords {
        self.info.pos.get_or_default().into()
    }

    pub fn shape(&self) -> Box3D<3, bool> {
        shape::box_from_fn(|_, _, _| {
            rand::thread_rng().gen_ratio(self.info.density().abs().min(100).max(0) as u32, 200)
        })
    }
}

impl CollectVoxels for Flow {
    fn collect_voxels<'a>(&'a self, _map: &crate::map::Map) -> Vec<Voxel<'a>> {
        let coords = self.coords();
        let shape = self.shape();
        (0_usize..3_usize)
            .flat_map(move |x| {
                (0_usize..3_usize).flat_map(move |y| {
                    (0_usize..3_usize).filter_map(move |z| {
                        if shape[2 - z][y][x] {
                            Some((x, y, z))
                        } else {
                            None
                        }
                    })
                })
            })
            .map(|(local_x, local_y, local_z)| {
                Coords::new(
                    coords.x * 3 + local_x as i32,
                    coords.y * 3 + local_y as i32,
                    coords.z * 3 + local_z as i32,
                )
            })
            .map(|coords| Voxel::new(coords, &self.material))
            .collect_vec()
    }
}
