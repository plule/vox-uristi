use dfhack_remote::{MatPair, MaterialDefinition};
use num_enum::IntoPrimitive;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use strum::{EnumCount, EnumIter, IntoEnumIterator};
use vox_writer::VoxWriter;

/// A material to be exported as an entry in the palette
#[derive(Debug, Clone)]
pub enum Material {
    /// Default material with hard-coded color
    Default(DefaultMaterials),
    /// Generic material built procedurally from Dwarf Fortress
    Generic(MatPair),
}

// temp for trees
#[derive(Debug, Clone)]
pub struct RandomMaterial {
    pub materials: Vec<Material>,
}

impl RandomMaterial {
    pub fn new(materials: Vec<Material>) -> Self {
        Self { materials }
    }

    pub fn pick(&self) -> &Material {
        self.materials.choose(&mut rand::thread_rng()).unwrap()
    }
}

impl From<Material> for RandomMaterial {
    fn from(value: Material) -> Self {
        Self {
            materials: vec![value],
        }
    }
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
    pub fn get_palette_color(&mut self, material: &Material) -> u8 {
        1 + match material {
            Material::Default(default_mat) => (*default_mat).into(),
            Material::Generic(mat_pair) => {
                let palette_size = self.colors.len();
                *self.colors.entry(mat_pair.to_owned()).or_insert_with(|| {
                    (DefaultMaterials::COUNT + palette_size)
                        .try_into()
                        .unwrap_or_default() // would be nice to warn in case of palette overflow
                })
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
