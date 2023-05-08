use crate::rfr::RGBColor;
use dfhack_remote::{MatPair, MaterialDefinition};
use num_enum::{FromPrimitive, IntoPrimitive};
use palette::{named, Srgb};
use std::collections::HashMap;
use strum::{EnumCount, EnumIter};
use vox_writer::VoxWriter;

/// A material to be exported as an entry in the palette
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Material {
    /// Default colors for which Dwarf Fortress does not give indication (water, magma, smoke...)
    Default(DefaultMaterials),
    /// 16 colors defined by Dwarf Fortress for console colors
    Console(ConsoleColor),
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
    DeadGrass,
}

#[derive(Debug, Clone, Copy, FromPrimitive, Hash, PartialEq, Eq)]
#[repr(i32)]
pub enum ConsoleColor {
    #[default]
    Black,
    Blue,
    Green,
    Cyan,
    Red,
    Magenta,
    Brown,
    Grey,
    DarkGrey,
    LightBlue,
    LightGreen,
    LightCyan,
    LightRed,
    LightMagenta,
    Yellow,
    White,
}

impl From<ConsoleColor> for palette::Srgb<u8> {
    fn from(value: ConsoleColor) -> Self {
        match value {
            ConsoleColor::Black => named::BLACK,
            ConsoleColor::Blue => named::BLUE,
            ConsoleColor::Green => named::GREEN,
            ConsoleColor::Cyan => named::CYAN,
            ConsoleColor::Red => named::DARKRED,
            ConsoleColor::Magenta => named::DARKMAGENTA,
            ConsoleColor::Brown => named::BROWN,
            ConsoleColor::Grey => named::GRAY,
            ConsoleColor::DarkGrey => named::DARKGRAY,
            ConsoleColor::LightBlue => named::LIGHTBLUE,
            ConsoleColor::LightGreen => named::LIGHTGREEN,
            ConsoleColor::LightCyan => named::LIGHTCYAN,
            ConsoleColor::LightRed => named::RED,
            ConsoleColor::LightMagenta => named::MAGENTA,
            ConsoleColor::Yellow => named::YELLOW,
            ConsoleColor::White => named::WHITE,
        }
    }
}

pub trait RGBAColor {
    fn get_rgba(&self) -> (u8, u8, u8, u8);
}

impl<T: RGBColor> RGBAColor for T {
    fn get_rgba(&self) -> (u8, u8, u8, u8) {
        let rgb = self.get_rgb();
        (rgb.0, rgb.1, rgb.2, 255)
    }
}

impl Material {
    pub fn get_color(&self, materials: &[MaterialDefinition]) -> (u8, u8, u8, u8) {
        match self {
            Material::Default(default) => default.get_rgba(),
            Material::Generic(matpair) => materials
                .iter()
                .find(|m| matpair == m.mat_pair.get_or_default())
                .map_or((0, 0, 0, 0), |material| {
                    let rgb = material.state_color.get_rgba();
                    (rgb.0, rgb.1, rgb.2, 255)
                }),
            Material::Console(console_color) => console_color.get_rgba(),
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
        }
    }
}

impl RGBColor for ConsoleColor {
    fn get_rgb(&self) -> (u8, u8, u8) {
        let color = Srgb::<u8>::from(*self);
        (color.red, color.green, color.blue)
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
