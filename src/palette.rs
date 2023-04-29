use dfhack_remote::{MatPair, MaterialDefinition};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use vox_writer::VoxWriter;

const HIDDEN: u8 = 0;
const WATER: u8 = 1;
const MAGMA: u8 = 2;
const DARK_GRASS: u8 = 3;
const LIGHT_GRASS: u8 = 4;

#[derive(Default)]
pub struct Palette {
    pub colors: HashMap<MatPair, u8>,
}

impl Palette {
    pub fn build_palette<'a>(&mut self, materials: impl Iterator<Item = &'a Material>) {
        self.colors.clear();
        for material in materials {
            for mat_pair in material.list_mat_pairs() {
                let palette_size = self.colors.len() as u8;
                self.colors
                    .entry(mat_pair.to_owned())
                    .or_insert_with(|| palette_size + 5);
            }
        }
    }

    pub fn write_palette(&self, vox: &mut VoxWriter, materials: &[MaterialDefinition]) {
        vox.clear_colors();
        vox.add_color(0, 0, 0, 255, HIDDEN);
        vox.add_color(0, 0, 255, 64, WATER);
        vox.add_color(255, 0, 0, 64, MAGMA);
        vox.add_color(0, 102, 0, 255, DARK_GRASS);
        vox.add_color(0, 153, 51, 255, LIGHT_GRASS);

        // Custom colored
        for (matpair, index) in self.colors.iter() {
            let material = materials
                .iter()
                .find(|m| matpair == m.mat_pair.get_or_default());
            if let Some(material) = material {
                let color = &material.state_color;
                vox.add_color(
                    color.red().try_into().unwrap_or_default(),
                    color.green().try_into().unwrap_or_default(),
                    color.blue().try_into().unwrap_or_default(),
                    255,
                    *index,
                );
            }
        }
    }
}

#[derive(Debug)]
pub enum Material {
    Hidden,
    Water,
    Magma,
    DarkGrass,
    LightGrass,
    Generic(MatPair),
    Random(Vec<MatPair>),
}

impl Material {
    pub fn list_mat_pairs(&self) -> Vec<MatPair> {
        match self {
            Material::Random(matpairs) => matpairs.clone(),
            Material::Generic(matpair) => vec![matpair.clone()],
            _ => vec![],
        }
    }

    #[allow(clippy::mutable_key_type)] // possibly an actual issue?
    pub fn pick_color(&self, palette: &HashMap<MatPair, u8>) -> u8 {
        (match self {
            Material::Hidden => HIDDEN,
            Material::Water => WATER,
            Material::Magma => MAGMA,
            Material::DarkGrass => DARK_GRASS,
            Material::LightGrass => LIGHT_GRASS,
            Material::Generic(pair) => *palette.get(pair).unwrap(),
            Material::Random(s) => {
                let mut rng = rand::thread_rng();
                let pair = s.choose(&mut rng).unwrap();
                *palette.get(pair).unwrap()
            }
        }) + 1
    }
}
