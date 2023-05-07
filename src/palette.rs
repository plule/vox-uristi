use dfhack_remote::{MatPair, MaterialDefinition};
use num_enum::IntoPrimitive;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use strum::{EnumCount, EnumIter};
use vox_writer::VoxWriter;

/// A material to be exported as an entry in the palette
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Material {
    /// Default material with hard-coded color
    Default(DefaultMaterials),
    /// Generic material built procedurally from Dwarf Fortress
    Generic(MatPair),
}

/// The default hard-coded materials
#[derive(Debug, Clone, Copy, IntoPrimitive, EnumIter, EnumCount, Hash, PartialEq, Eq)]
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

impl Material {
    pub fn get_color(&self, materials: &[MaterialDefinition]) -> (u8, u8, u8, u8) {
        match self {
            Material::Default(default) => default.get_color(),
            Material::Generic(matpair) => materials
                .iter()
                .find(|m| matpair == m.mat_pair.get_or_default())
                .map_or((0, 0, 0, 0), |material| {
                    let color = &material.state_color;
                    (
                        color.red().try_into().unwrap_or_default(),
                        color.green().try_into().unwrap_or_default(),
                        color.blue().try_into().unwrap_or_default(),
                        255,
                    )
                }),
        }
    }
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
    pub materials: HashMap<Material, u8>,
}

impl Palette {
    pub fn get_palette_color(&mut self, material: &Material) -> u8 {
        let palette_size = self.materials.len();
        1 + *self
            .materials
            .entry(material.clone())
            .or_insert_with(|| palette_size.try_into().unwrap_or_default()) // would be nice to warn in case of palette overflow
    }

    pub fn write_palette(&self, vox: &mut VoxWriter, materials: &[MaterialDefinition]) {
        vox.clear_colors();
        for (material, index) in &self.materials {
            let (r, g, b, a) = material.get_color(materials);
            vox.add_color(r, g, b, a, *index);
        }
    }
}
