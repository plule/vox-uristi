use crate::dot_vox_builder::MaterialExt;
use crate::rfr::RGBColor;
use dfhack_remote::{core_text_fragment::Color, MatPair, MaterialDefinition};
use dot_vox::DotVoxData;
use num_enum::IntoPrimitive;
use palette::{named, rgb::Rgb, FromColor, Hsv};
use std::collections::HashMap;
use strum::{EnumCount, EnumIter};

/// A material to be exported as an entry in the palette
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Material {
    /// Default colors for which Dwarf Fortress does not give indication (water, magma, smoke...)
    Default(DefaultMaterials),
    /// Generic material built procedurally from Dwarf Fortress
    Generic(MatPair),
    /// Generic material with a growth console color associated to it
    Plant {
        material: MatPair,
        source_color: Color,
        dest_color: Color,
    },
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
    DeadGrass,
    Wood,
}

pub trait RGBAColor {
    fn get_rgba(&self) -> (u8, u8, u8, u8);
}

impl<T: RGBColor> RGBAColor for T {
    fn get_rgba(&self) -> (u8, u8, u8, u8) {
        let rgb = self.rgb();
        (rgb.red, rgb.green, rgb.blue, 255)
    }
}

impl Material {
    pub fn get_material(
        &self,
        materials: &[MaterialDefinition],
        id: u32,
    ) -> ((u8, u8, u8, u8), dot_vox::Material) {
        match self {
            Material::Default(default) => {
                let color = default.get_rgba();
                let mat = match default {
                    DefaultMaterials::Water => dot_vox::Material::glass(id, 0.1, 0.3, 0.2, 0.05),
                    DefaultMaterials::Magma => dot_vox::Material::emit(id, 0.5, 2, 0.0),
                    DefaultMaterials::Fire => dot_vox::Material::emit(id, 0.5, 2, 0.0),
                    _ => dot_vox::Material::diffuse(id),
                };

                (color, mat)
            }
            Material::Generic(matpair) => {
                let color = materials
                    .iter()
                    .find(|m| matpair == m.mat_pair.get_or_default())
                    .map_or((0, 0, 0, 0), |material| material.state_color.get_rgba());
                let material = match (matpair.mat_type(), matpair.mat_index()) {
                    // Metals
                    (0, i) if i <= 25 => dot_vox::Material::metal(id, 1.0, 0.1, 0.3, 0.05),
                    // Green, clear and crystal glass
                    (3 | 4 | 5, _) => dot_vox::Material::glass(id, 0.1, 0.3, 0.36, 0.5),
                    // Marble
                    (0, 185) => dot_vox::Material::metal(id, 0.61, 0.8, 0.3, 0.0),

                    _ => dot_vox::Material::diffuse(id),
                };
                (color, material)
            }
            Material::Plant {
                material,
                source_color,
                dest_color,
            } => {
                let mat = dot_vox::Material::diffuse(id);
                let main_color = materials
                    .iter()
                    .find(|m| material == m.mat_pair.get_or_default())
                    .map_or(named::BLACK, |material| material.state_color.rgb());
                if source_color == dest_color {
                    return (
                        (main_color.red, main_color.green, main_color.blue, 255),
                        mat,
                    );
                }
                let mut color = Hsv::from_color(main_color.into_linear::<f32>());
                let source_color = Hsv::from_color(source_color.rgb().into_linear::<f32>());
                let dest_color = Hsv::from_color(dest_color.rgb().into_linear::<f32>());
                // I have no idea what's going on here, I just did my best to replicate what is done in Armok Vision
                // https://github.com/RosaryMala/armok-vision/blob/3027c785a54d7a8d9a7a9f7f2a10a1815c3bb500/Assets/Scripts/MapGen/DfColor.cs#L37
                // and the result looks fairly similar to in-game colors.
                color.hue += dest_color.hue - source_color.hue;
                if source_color.value > dest_color.value {
                    color.value *= dest_color.value / source_color.value;
                } else {
                    color.value = 1.0
                        - ((1.0 - color.value)
                            * ((1.0 - dest_color.value) / (1.0 - source_color.value)));
                }
                let color = Rgb::from_color(color);
                let color: Rgb<palette::encoding::Srgb, u8> = Rgb::from_linear(color);
                ((color.red, color.green, color.blue, 255), mat)
            }
        }
    }
}

impl RGBAColor for DefaultMaterials {
    fn get_rgba(&self) -> (u8, u8, u8, u8) {
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
            DefaultMaterials::DeadGrass => (102, 102, 0, 255),
            DefaultMaterials::Wood => (75, 21, 0, 255),
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
        *self
            .materials
            .entry(material.clone())
            .or_insert_with(|| palette_size.try_into().unwrap_or_default()) // would be nice to warn in case of palette overflow
    }

    pub fn write_palette(&self, vox: &mut DotVoxData, materials: &[MaterialDefinition]) {
        for (material, index) in &self.materials {
            let ((r, g, b, a), material) = material.get_material(materials, u32::from(*index) + 1);
            vox.palette[*index as usize] = dot_vox::Color { r, g, b, a };
            vox.materials[*index as usize] = material;
        }
    }
}
