//! Material and palette management

use crate::export::context::DFContext;
use crate::rfr::RGBColor;
use crate::{dot_vox_builder::MaterialExt, rfr::BasicMaterialInfoExt};
use dfhack_remote::TiletypeMaterial;
use dfhack_remote::{core_text_fragment::Color, MatPair};
use dot_vox::DotVoxData;
use num_enum::IntoPrimitive;
use palette::encoding::Linear;
use palette::{named, rgb::Rgb, FromColor, Hsv};
use palette::{Darken, Desaturate};
use std::collections::HashMap;
use strum::{EnumCount, EnumIter, IntoEnumIterator};

/// A material to be exported as an entry in the palette
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Material {
    /// Default colors for which Dwarf Fortress does not give indication (water, magma, smoke...)
    Default(DefaultMaterials),
    /// Generic material built procedurally from Dwarf Fortress
    Generic(MatPair),
    /// Darker variant of a generic material
    DarkGeneric(MatPair),
    /// Generic material with tile information
    TileGeneric(MatPair, TiletypeMaterial),
    DarkTileGeneric(MatPair, TiletypeMaterial),
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
    Plant,
    DeadPlant,
    Wood,
    Light,
    Rock,
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

impl RGBAColor for DefaultMaterials {
    fn get_rgba(&self) -> (u8, u8, u8, u8) {
        match self {
            DefaultMaterials::Hidden => (0, 0, 0, 255),
            DefaultMaterials::Water => (0, 0, 255, 64),
            DefaultMaterials::Mist => (255, 255, 255, 64),
            DefaultMaterials::Magma => (134, 0, 0, 64),
            DefaultMaterials::Fire => (255, 174, 0, 64),
            DefaultMaterials::Smoke => (100, 100, 100, 64),
            DefaultMaterials::Miasma => (208, 89, 255, 64),
            DefaultMaterials::Wood => (75, 21, 0, 255),
            DefaultMaterials::Light => (255, 255, 255, 255),
            DefaultMaterials::Rock => (100, 98, 122, 255),
            DefaultMaterials::Plant => (0, 153, 51, 255),
            DefaultMaterials::DeadPlant => (61, 102, 0, 255),
        }
    }
}

#[derive(Default)]
pub struct Palette {
    /// Effective palette. Two different DF material are the same
    /// effective material if they have the same characteristics in .vox
    pub materials: HashMap<EffectiveMaterial, u8>,
    /// Cache to avoid building the EffectiveMaterial for each voxel
    pub material_cache: HashMap<Material, u8>,
}

impl Palette {
    pub fn get(&mut self, material: &Material, context: &DFContext) -> u8 {
        if let Some(from_cache) = self.material_cache.get(material) {
            return *from_cache;
        }

        let palette_size = self.materials.len();
        let effective_material = EffectiveMaterial::from_material(material, context);
        let color = *self.materials.entry(effective_material).or_insert_with(|| {
            // would be nice to warn in case of palette overflow
            palette_size
                .min(u8::MAX as usize - 1)
                .try_into()
                .unwrap_or_default()
        });
        self.material_cache.insert(material.clone(), color);
        color
    }

    pub fn cache_default_materials(&mut self, context: &DFContext) {
        for default_material in DefaultMaterials::iter() {
            let material = Material::Default(default_material);
            self.get(&material, context);
        }
    }

    pub fn write_palette(&self, vox: &mut DotVoxData) {
        for (material, index) in &self.materials {
            material.apply_material(
                &mut vox.palette[*index as usize],
                &mut vox.materials[*index as usize + 1],
            );
        }
    }
}

/// Intermediary hashable material format to group together
/// material that are the same from different sources
#[derive(Hash, PartialEq, Eq, Default, Clone)]
pub struct EffectiveMaterial {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
    pub mat_type: Option<&'static str>,
    pub metalness: Option<u8>,
    pub roughness: Option<u8>,
    pub transparency: Option<u8>,
    pub emit: Option<u8>,
    pub flux: Option<u8>,
    pub ior: Option<u8>,
    pub media_type: Option<&'static str>,
    pub density: Option<u8>,
}

impl EffectiveMaterial {
    pub fn from_material(material: &Material, context: &DFContext) -> Self {
        match material {
            Material::Default(default) => {
                let mut res = EffectiveMaterial::default();
                (res.r, res.g, res.b, res.a) = default.get_rgba();
                match default {
                    DefaultMaterials::Water => {
                        res.mat_type = Some("_glass");
                        res.transparency = Some(50);
                    }
                    DefaultMaterials::Magma => {
                        res.mat_type = Some("_blend");
                        res.roughness = Some(100);
                        res.ior = Some(0);
                        res.metalness = Some(50);
                        res.transparency = Some(100);
                        res.media_type = Some("2"); // emit
                        res.density = Some(100);
                    }
                    DefaultMaterials::Fire => {
                        res.mat_type = Some("_emit");
                        res.emit = Some(50);
                        res.flux = Some(1);
                    }
                    DefaultMaterials::Mist => {
                        res.mat_type = Some("_glass");
                        res.ior = Some(0);
                        res.transparency = Some(75);
                    }
                    DefaultMaterials::Smoke | DefaultMaterials::Miasma => {
                        res.mat_type = Some("_glass");
                        res.ior = Some(0);
                        res.transparency = Some(25);
                    }
                    DefaultMaterials::Light => {
                        res.mat_type = Some("_emit");
                        res.emit = Some(50);
                        res.flux = Some(1);
                    }
                    _ => {
                        res.mat_type = Some("_diffuse");
                    }
                };
                res
            }
            Material::Generic(matpair) => Self::from_matpair(matpair, context),
            Material::DarkGeneric(matpair) => {
                let mut res = Self::from_matpair(matpair, context);
                res.set_hsv(res.hsv().darken(0.2));
                res
            }
            Material::TileGeneric(matpair, tiletype_material) => {
                Self::from_matpair_and_tiletype(matpair, tiletype_material, context)
            }
            Material::DarkTileGeneric(matpair, tiletype_material) => {
                let mut res = Self::from_matpair_and_tiletype(matpair, tiletype_material, context);
                res.set_hsv(res.hsv().darken(0.2));
                res
            }
            Material::Plant {
                material: mat,
                source_color,
                dest_color,
            } => {
                let mut res = EffectiveMaterial {
                    mat_type: Some("_diffuse"),
                    ..Default::default()
                };
                let main_color = context
                    .materials
                    .material_list
                    .iter()
                    .find(|m| mat == m.mat_pair.get_or_default())
                    .map_or(named::BLACK, |material| material.state_color.rgb());
                if source_color == dest_color {
                    (res.r, res.g, res.b, res.a) =
                        (main_color.red, main_color.green, main_color.blue, 255);
                    return res;
                }
                let mut hsv = Hsv::from_color(main_color.into_linear::<f64>());
                let source_color = Hsv::from_color(source_color.rgb().into_linear::<f64>());
                let dest_color = Hsv::from_color(dest_color.rgb().into_linear::<f64>());
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
                res.set_hsv(hsv);
                res
            }
        }
    }

    pub fn from_matpair(matpair: &MatPair, context: &DFContext) -> Self {
        let mut res = EffectiveMaterial::default();
        (res.r, res.g, res.b, res.a) = (0, 0, 0, 0);
        if let Some(material) = context
            .materials
            .material_list
            .iter()
            .find(|m| matpair == m.mat_pair.get_or_default())
        {
            // raw rgba
            (res.r, res.g, res.b, res.a) = material.state_color.get_rgba();

            // override for water ("clear" so no color)
            if material.id() == "WATER" {
                (res.r, res.g, res.b, res.a) = (200, 200, 230, 255);
            }
        }
        if let Some(info) = context
            .inorganic_materials_map
            .get(&(matpair.mat_type(), matpair.mat_index()))
        {
            for flag in info.flag_names(&context.enums) {
                match flag {
                    "IS_METAL" => {
                        res.mat_type = Some("_metal");
                        res.metalness = Some(60);
                        res.roughness = Some(20);
                    }
                    "IS_GEM" => {
                        res.mat_type = Some("_glass");
                        res.roughness = Some(3);
                        res.transparency = Some(30);
                    }
                    "IS_GLASS" => {
                        res.mat_type = Some("_glass");
                        res.roughness = Some(5);
                        res.transparency = Some(60);
                    }
                    "IS_CERAMIC" => {
                        res.mat_type = Some("_glass");
                        res.transparency = Some(0);
                    }
                    _ => {}
                }
            }
            if info.token() == "MARBLE" {
                res.mat_type = Some("_metal");
                res.roughness = Some(50);
                res.metalness = Some(50);
            }
        }
        res
    }

    pub fn from_matpair_and_tiletype(
        matpair: &MatPair,
        tiletype_material: &TiletypeMaterial,
        context: &DFContext,
    ) -> Self {
        let mut res = Self::from_matpair(matpair, context);
        match tiletype_material {
            // Apply a reflective effect on ice
            TiletypeMaterial::FROZEN_LIQUID => {
                res.mat_type = Some("_glass");
                res.ior = Some(50);
                res.transparency = Some(50);
            }
            // Obsidian and friends
            TiletypeMaterial::LAVA_STONE => {
                res.mat_type = Some("_glass");
                res.roughness = Some(10);
                res.transparency = Some(0);
                res.ior = Some(5);
            }
            // Grass don't have a proper material, or maybe hidden into plants raws?
            TiletypeMaterial::GRASS_LIGHT => {
                (res.r, res.g, res.b, res.a) = (0, 153, 51, 255);
            }
            TiletypeMaterial::GRASS_DARK => {
                (res.r, res.g, res.b, res.a) = (0, 102, 0, 255);
            }
            TiletypeMaterial::GRASS_DRY | TiletypeMaterial::GRASS_DEAD => {
                (res.r, res.g, res.b, res.a) = (61, 102, 0, 255);
            }
            // Desaturate a bit constructions
            TiletypeMaterial::CONSTRUCTION => {
                res.set_hsv(res.hsv().desaturate(0.1));
            }
            // Desaturate more raw materials
            TiletypeMaterial::STONE | TiletypeMaterial::DRIFTWOOD => {
                res.set_hsv(res.hsv().desaturate(0.15));
            }
            _ => {}
        }
        let mut hsv = res.hsv();
        // Avoid pitch black materials
        if hsv.value < 0.02 {
            hsv.value = 0.02;
            res.set_hsv(hsv);
        }
        res
    }
    fn apply_material(&self, color: &mut dot_vox::Color, material: &mut dot_vox::Material) {
        let Self {
            r,
            g,
            b,
            a,
            mat_type,
            metalness,
            roughness,
            transparency,
            emit,
            flux,
            ior,
            media_type,
            density,
        } = self.to_owned();
        color.r = r;
        color.g = g;
        color.b = b;
        color.a = a;
        if let Some(mat_type) = mat_type {
            material.set_type(mat_type);
        }
        if let Some(emit) = emit {
            material.set_emit((emit as f32) / 100.0);
        }

        if let Some(metalness) = metalness {
            material.set_metalness((metalness as f32) / 100.0);
        }

        if let Some(roughness) = roughness {
            material.set_roughness((roughness as f32) / 100.0);
        }

        if let Some(transparency) = transparency {
            material.set_transparency((transparency as f32) / 100.0);
        }

        if let Some(flux) = flux {
            material.set_flux(flux as f32);
        }

        if let Some(ior) = ior {
            material.set_ior((ior as f32) / 100.0);
        }

        if let Some(media_type) = media_type {
            material.set_media_type(media_type);
        }

        if let Some(density) = density {
            material.set_density((density as f32) / 1000.0);
        }
    }

    fn rgb(&self) -> palette::Srgb<u8> {
        (self.r, self.g, self.b).into()
    }

    fn set_rgb(&mut self, rgb: palette::Srgb<u8>) {
        (self.r, self.g, self.b) = rgb.into();
    }

    fn hsv(&self) -> Hsv<Linear<palette::encoding::Srgb>, f64> {
        Hsv::from_color(self.rgb().into_linear())
    }

    fn set_hsv(&mut self, hsv: Hsv<Linear<palette::encoding::Srgb>, f64>) {
        let rgb = Rgb::from_linear(Rgb::from_color(hsv));
        self.set_rgb(rgb);
    }
}
