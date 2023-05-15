use crate::rfr::{MaterialFlags, RGBColor};
use crate::{dot_vox_builder::MaterialExt, rfr::BasicMaterialInfoExt};
use dfhack_remote::{core_text_fragment::Color, BasicMaterialInfo, MatPair, MaterialDefinition};
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
    pub fn apply_material(
        &self,
        materials: &[MaterialDefinition],
        material_info: &HashMap<(i32, i32), &BasicMaterialInfo>,
        color: &mut dot_vox::Color,
        material: &mut dot_vox::Material,
    ) {
        match self {
            Material::Default(default) => {
                (color.r, color.g, color.b, color.a) = default.get_rgba();
                match default {
                    DefaultMaterials::Water => {
                        material.set_glass();
                        material.set_transparency(0.5);
                    }
                    DefaultMaterials::Magma => {
                        material.set_emissive();
                        material.set_emit(0.5);
                        material.set_flux(2.0);
                    }
                    DefaultMaterials::Fire => {
                        material.set_emissive();
                        material.set_emit(0.5);
                        material.set_flux(2.0);
                    }
                    DefaultMaterials::Mist => {
                        material.set_glass();
                        material.set_ior(0.0);
                        material.set_transparency(0.75);
                    }
                    DefaultMaterials::Smoke => {
                        material.set_glass();
                        material.set_ior(0.0);
                        material.set_transparency(0.25);
                    }
                    _ => {
                        material.set_diffuse();
                    }
                };
            }
            Material::Generic(matpair) => {
                (color.r, color.g, color.b, color.a) = materials
                    .iter()
                    .find(|m| matpair == m.mat_pair.get_or_default())
                    .map_or((0, 0, 0, 0), |material| material.state_color.get_rgba());
                if let Some(info) = material_info.get(&(matpair.mat_type(), matpair.mat_index())) {
                    for flag in info.get_flags() {
                        match flag {
                            MaterialFlags::IsMetal => {
                                material.set_metal();
                                material.set_metalness(1.0);
                            }
                            MaterialFlags::IsGem => {
                                material.set_glass();
                                material.set_roughness(0.025);
                                material.set_transparency(0.3);
                            }
                            MaterialFlags::IsGlass => {
                                material.set_glass();
                                material.set_roughness(0.05);
                                material.set_transparency(0.6);
                            }
                            MaterialFlags::IsCeramic => {
                                material.set_glass();
                                material.set_transparency(0.0);
                            }
                            _ => {}
                        }
                    }
                    if info.token() == "MARBLE" {
                        material.set_metal();
                        material.set_roughness(0.5);
                        material.set_metalness(0.5);
                    }
                }
            }
            Material::Plant {
                material: mat,
                source_color,
                dest_color,
            } => {
                material.set_diffuse();
                let main_color = materials
                    .iter()
                    .find(|m| mat == m.mat_pair.get_or_default())
                    .map_or(named::BLACK, |material| material.state_color.rgb());
                if source_color == dest_color {
                    (color.r, color.g, color.b, color.a) =
                        (main_color.red, main_color.green, main_color.blue, 255);
                    return;
                }
                let mut hsv = Hsv::from_color(main_color.into_linear::<f32>());
                let source_color = Hsv::from_color(source_color.rgb().into_linear::<f32>());
                let dest_color = Hsv::from_color(dest_color.rgb().into_linear::<f32>());
                // I have no idea what's going on here, I just did my best to replicate what is done in Armok Vision
                // https://github.com/RosaryMala/armok-vision/blob/3027c785a54d7a8d9a7a9f7f2a10a1815c3bb500/Assets/Scripts/MapGen/DfColor.cs#L37
                // and the result looks fairly similar to in-game colors.
                hsv.hue += dest_color.hue - source_color.hue;
                if source_color.value > dest_color.value {
                    hsv.value *= dest_color.value / source_color.value;
                } else {
                    hsv.value = 1.0
                        - ((1.0 - hsv.value)
                            * ((1.0 - dest_color.value) / (1.0 - source_color.value)));
                }
                let rgb = Rgb::from_color(hsv);
                let rgba: Rgb<palette::encoding::Srgb, u8> = Rgb::from_linear(rgb);
                (color.r, color.g, color.b, color.a) = (rgba.red, rgba.green, rgba.blue, 255);
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

    pub fn write_palette(
        &self,
        vox: &mut DotVoxData,
        materials: &[MaterialDefinition],
        material_info: &HashMap<(i32, i32), &BasicMaterialInfo>,
    ) {
        for (material, index) in &self.materials {
            material.apply_material(
                materials,
                material_info,
                &mut vox.palette[*index as usize],
                &mut vox.materials[*index as usize + 1],
            );
        }
    }
}
