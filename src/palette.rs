use dfhack_remote::{MatPair, MaterialDefinition};
use num_enum::IntoPrimitive;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use strum::{EnumCount, EnumIter, IntoEnumIterator};
use vox_writer::VoxWriter;

/// A material to be exported as an entry in the palette
#[derive(Debug)]
pub enum Material {
    /// Default material with hard-coded color
    Default(DefaultMaterials),
    /// Generic material built procedurally from Dwarf Fortress
    Generic(MatPair),
    /// Generic material randomly picked from a list
    Random(Vec<MatPair>),
}

/// The default hard-coded materials
#[derive(Debug, Clone, Copy, IntoPrimitive, EnumIter, EnumCount)]
#[repr(u8)]
pub enum DefaultMaterials {
    /// Common material for all hidden tiles
    Hidden,
    Water,
    Mist,
    Magma,
    Fire,
    Smoke,
    Miasma,
    DarkGrass,
    LightGrass,
}

impl DefaultMaterials {
    pub fn get_color(&self) -> (u8, u8, u8, u8) {
        match self {
            DefaultMaterials::Hidden => (0, 0, 0, 255),
            DefaultMaterials::Water => (0, 0, 255, 64),
            DefaultMaterials::Mist => (255, 255, 255, 64),
            DefaultMaterials::Magma => (255, 0, 0, 64),
            DefaultMaterials::Fire => (255, 174, 0, 64),
            DefaultMaterials::Smoke => (100, 100, 100, 64),
            DefaultMaterials::Miasma => (208, 89, 255, 64),
            DefaultMaterials::DarkGrass => (0, 102, 0, 255),
            DefaultMaterials::LightGrass => (0, 153, 51, 255),
        }
    }
}

#[derive(Default)]
pub struct Palette {
    pub colors: HashMap<MatPair, u8>,
}

impl Palette {
    pub fn build_palette<'a>(&mut self, materials: impl Iterator<Item = &'a Material>) {
        self.colors.clear();
        for material in materials {
            for mat_pair in material.list_mat_pairs() {
                let palette_size = self.colors.len();
                self.colors.entry(mat_pair.to_owned()).or_insert_with(|| {
                    (DefaultMaterials::COUNT + palette_size)
                        .try_into()
                        .unwrap_or_default() // would be nice to warn in case of palette overflow
                });
            }
        }
    }

    pub fn write_palette(&self, vox: &mut VoxWriter, materials: &[MaterialDefinition]) {
        vox.clear_colors();
        for default_mat in DefaultMaterials::iter() {
            let (r, g, b, a) = default_mat.get_color();
            vox.add_color(r, g, b, a, default_mat.into());
        }

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
            Material::Default(material) => Into::<u8>::into(*material),
            Material::Generic(pair) => *palette.get(pair).unwrap(),
            Material::Random(s) => {
                let mut rng = rand::thread_rng();
                let pair = s.choose(&mut rng).unwrap();
                *palette.get(pair).unwrap()
            }
        }) + 1
    }
}
