use crate::{tile::Tile, tile_iterator::MatPairHash};
use dfhack_remote::MaterialDefinition;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use vox_writer::VoxWriter;

const WATER: u8 = 0;
const MAGMA: u8 = 1;
const DARK_GRASS: u8 = 2;
const LIGHT_GRASS: u8 = 3;

#[derive(Default)]
pub struct Palette {
    pub colors: HashMap<MatPairHash, u8>,
}

impl Palette {
    pub fn build_palette<'a>(&mut self, tiles: impl Iterator<Item = &'a Tile>) {
        self.colors.clear();
        for tile in tiles {
            for mat_pair in tile.material.list_mat_pairs() {
                let palette_size = self.colors.len() as u8;
                self.colors
                    .entry(mat_pair.to_owned())
                    .or_insert_with(|| palette_size + 4);
            }
        }
    }

    pub fn write_palette(&self, vox: &mut VoxWriter, materials: &[MaterialDefinition]) {
        vox.clear_colors();
        vox.add_color(0, 0, 255, 64, WATER);
        vox.add_color(255, 0, 0, 64, MAGMA);
        vox.add_color(0, 102, 0, 255, DARK_GRASS);
        vox.add_color(0, 153, 51, 255, LIGHT_GRASS);

        // Custom colored
        for (matpair, index) in self.colors.iter() {
            let material = materials.iter().find(|m| {
                matpair.mat_index == m.mat_pair.mat_index()
                    && matpair.mat_type == m.mat_pair.mat_type()
            });
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
    Water,
    Magma,
    DarkGrass,
    LightGrass,
    Generic(Vec<MatPairHash>),
}

impl Material {
    pub fn list_mat_pairs(&self) -> Vec<MatPairHash> {
        match self {
            Material::Generic(matpairs) => matpairs.clone(),
            _ => vec![],
        }
    }

    pub fn pick_color(&self, palette: &HashMap<MatPairHash, u8>) -> u8 {
        (match self {
            Material::Water => WATER,
            Material::Magma => MAGMA,
            Material::DarkGrass => DARK_GRASS,
            Material::LightGrass => LIGHT_GRASS,
            Material::Generic(s) => {
                let mut rng = rand::thread_rng();
                let pair = s.choose(&mut rng).unwrap();
                *palette.get(pair).unwrap()
            }
        }) + 1
    }
}
