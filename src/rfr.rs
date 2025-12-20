//! Remote fortress reader API helpers
use crate::{
    coords::{DFBlockCoords, DFLocalCoords, WithBlockCoords},
    DFMapCoords,
};
use anyhow::Result;
use bitflags::bitflags;
use dfhack_remote::{
    core_text_fragment::Color, BasicMaterialInfo, BlockList, BlockRequest, BuildingDefinition,
    BuildingInstance, ColorDefinition, GrowthPrint, ListEnumsOut, MapBlock, MatPair, Spatter,
    Tiletype, TiletypeList, TreeGrowth,
};
use palette::{named, Srgb};
use protobuf::Enum;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    ops::{Range, RangeInclusive},
};

/// General DFHack remote helper extensions
#[easy_ext::ext(DFHackExt)]
pub impl dfhack_remote::Client {
    /// Offset between the z view position and the displayed elevation
    fn elevation_offset(&mut self) -> dfhack_remote::Result<i32> {
        let map_info = self.remote_fortress_reader().get_map_info()?;
        Ok(map_info.block_pos_z() - 100)
    }

    /// Get the current elevation as displayed in dwarf fortress
    fn elevation(&mut self) -> dfhack_remote::Result<i32> {
        let offset = self.elevation_offset()?;
        let view_info = self.remote_fortress_reader().get_view_info()?;
        Ok(view_info.view_pos_z() + offset)
    }

    #[cfg(feature = "dev")]
    fn set_elevation(&mut self, elevation: i32) -> dfhack_remote::Result<()> {
        let offset = self.elevation_offset()?;
        let scriptlet = format!(
            r#"df.global.window_z={}
df.global.game.minimap.mustmake=1
df.global.game.minimap.update=1"#,
            elevation - offset
        );
        let mut req = dfhack_remote::CoreRunCommandRequest::new();
        req.set_command("lua".to_string());
        req.arguments.push(scriptlet);
        self.core().run_command(req)?;
        Ok(())
    }
}

impl WithBlockCoords for MapBlock {
    fn block_coords(&self) -> DFBlockCoords {
        DFBlockCoords::new(self.map_x(), self.map_y(), self.map_z())
    }
}

/// Wrapper around dwarf fortress blocks to help access individual tile properties
#[derive(Debug)]
pub struct BlockTile<'a> {
    block: &'a MapBlock,
    index: usize,
    tiletypes: &'a TiletypeList,
    empty_spatters: Vec<Spatter>,
}

pub struct BlockListIterator<'a> {
    client: &'a mut dfhack_remote::Client,
    block_per_it: i32,
    x_range: Range<i32>,
    y_range: Range<i32>,
    z_range: Range<i32>,
    remaining: usize,
}

pub struct TileIterator<'a> {
    block: &'a MapBlock,
    index: Range<usize>,
    tiletypes: &'a TiletypeList,
}

pub trait RGBColor {
    fn rgb(&self) -> palette::Srgb<u8>;
}

pub trait ConsoleColor {
    fn get_console_color(&self) -> Color;
}

bitflags! {
    /// Building flags
    /// From https://github.com/DFHack/df-structures/blob/1f22dd8b8aa767609ea13bf1d2da8907001e0ce2/df.buildings.xml#L205
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct BuildingFlags: u32 {
        const EXISTS = 0b00000001;
        const SITE_BLOCKED = 0b00000010;
        const ROOM_COLLISION = 0b00000100;
        const UNK1 = 0b00001000;
        const ALMOST_DELETED = 0b00010000;
        const IN_UPDATE = 0b00100000;
        const FROM_WORLDGEN = 0b01000000;
    }
}

impl<'a> TileIterator<'a> {
    pub fn new(block: &'a MapBlock, tiletypes: &'a TiletypeList) -> Self {
        Self {
            block,
            index: 0..block.tiles.len(),
            tiletypes,
        }
    }
}

impl<'a> Iterator for TileIterator<'a> {
    type Item = BlockTile<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index.next();
        index.map(|index| BlockTile::new(self.block, index, self.tiletypes))
    }
}

impl<'a> BlockListIterator<'a> {
    pub fn try_new(
        client: &'a mut dfhack_remote::Client,
        block_per_it: i32,
        x_range: Range<i32>,
        y_range: Range<i32>,
        z_range: Range<i32>,
    ) -> Result<Self> {
        let map_info = client.remote_fortress_reader().get_map_info()?;
        let size_x = map_info.block_size_x() as usize;
        let size_y = map_info.block_size_y() as usize;
        let size_z = (z_range.end - z_range.start) as usize;
        let remaining = (size_x * size_y * size_z) / (block_per_it as usize);

        client.remote_fortress_reader().reset_map_hashes()?;
        Ok(Self {
            client,
            block_per_it,
            x_range,
            y_range,
            z_range,
            remaining,
        })
    }
}

impl<'a> Iterator for BlockListIterator<'a> {
    type Item = Result<BlockList>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut req = BlockRequest::new();
        req.set_blocks_needed(self.block_per_it);
        req.set_min_x(self.x_range.start);
        req.set_max_x(self.x_range.end);
        req.set_min_y(self.y_range.start);
        req.set_max_y(self.y_range.end);
        req.set_min_z(self.z_range.start);
        req.set_max_z(self.z_range.end);
        match self.client.remote_fortress_reader().get_block_list(req) {
            Ok(blocks) => {
                if blocks.map_blocks.iter().all(|b| b.tiles.is_empty()) {
                    // RFR will indefinitely stream block list for live view update
                    // Here we stop as soon as there is an empty block
                    return None;
                }
                self.remaining = self.remaining.saturating_sub(1);
                Some(Ok(blocks.reply))
            }
            Err(err) => Some(Err(err.into())),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, None)
    }
}

impl<'a> BlockTile<'a> {
    pub fn new(block: &'a MapBlock, index: usize, tiletypes: &'a TiletypeList) -> Self {
        Self {
            block,
            index,
            tiletypes,
            empty_spatters: Default::default(),
        }
    }

    pub fn local_coords(&self) -> DFLocalCoords {
        DFLocalCoords::from_index(self.index)
    }

    pub fn global_coords(&self) -> DFMapCoords {
        self.block.block_coords() + self.local_coords()
    }

    pub fn hidden(&self) -> bool {
        self.block.hidden[self.index]
    }

    pub fn water(&self) -> i32 {
        self.block.water[self.index]
    }

    pub fn tile_type_index(&self) -> i32 {
        self.block.tiles[self.index]
    }

    pub fn tile_type(&self) -> &Tiletype {
        &self.tiletypes.tiletype_list[self.tile_type_index() as usize]
    }

    pub fn material(&self) -> &MatPair {
        &self.block.materials[self.index]
    }

    pub fn base_material(&self) -> &MatPair {
        &self.block.base_materials[self.index]
    }

    pub fn vein_material(&self) -> &MatPair {
        &self.block.vein_materials[self.index]
    }

    pub fn magma(&self) -> i32 {
        self.block.magma[self.index]
    }

    pub fn water_stagnant(&self) -> bool {
        self.block.water_stagnant[self.index]
    }

    pub fn water_salt(&self) -> bool {
        self.block.water_salt[self.index]
    }

    pub fn tree(&self) -> DFMapCoords {
        DFMapCoords::new(
            self.block.tree_x[self.index],
            self.block.tree_y[self.index],
            self.block.tree_z[self.index],
        )
    }

    pub fn tree_origin(&self) -> DFMapCoords {
        let coord = self.global_coords();
        let tree = self.tree();
        DFMapCoords::new(coord.x - tree.x, coord.y - tree.y, coord.z + tree.z)
    }

    pub fn tree_percent(&self) -> i32 {
        self.block.tree_percent[self.index]
    }

    pub fn grass_percent(&self) -> i32 {
        self.block
            .grass_percent
            .get(self.index)
            .copied()
            .unwrap_or_default()
    }

    pub fn spatters(&self) -> &Vec<Spatter> {
        self.block
            .spatterPile
            .get(self.index)
            .map(|pile| &pile.spatters)
            .unwrap_or(&self.empty_spatters)
    }
}

impl Display for BlockTile<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "coords: {}", self.global_coords())?;
        writeln!(f, "hidden: {}", self.hidden())?;
        writeln!(f, "water: {}", self.water())?;
        writeln!(f, "tile_type: {}", self.tile_type())?;
        writeln!(f, "material: {}", self.material())?;
        writeln!(f, "base_material: {}", self.base_material())?;
        writeln!(f, "vein_material: {}", self.vein_material())?;
        writeln!(f, "magma: {}", self.magma())?;
        writeln!(f, "water_stagnant: {}", self.water_stagnant())?;
        writeln!(f, "water_salt: {}", self.water_salt())?;
        writeln!(f, "tree: {}", self.tree())?;
        writeln!(f, "tree_origin: {}", self.tree_origin())?;
        writeln!(f, "tree_percent: {}", self.tree_percent())?;
        writeln!(f, "grass: {}", self.grass_percent())?;
        for spatter in self.spatters() {
            writeln!(
                f,
                "spatter: {} ({}). state: {:?}. material: t{} i{}. item: t{} i{}",
                spatter.amount(),
                spatter.amount_normalized(),
                spatter.state(),
                spatter.material.get_or_default().mat_type(),
                spatter.material.get_or_default().mat_index(),
                spatter.item.get_or_default().mat_type(),
                spatter.item.get_or_default().mat_index(),
            )?;
        }
        Ok(())
    }
}

impl RGBColor for ColorDefinition {
    fn rgb(&self) -> Srgb<u8> {
        Srgb::new(
            self.red().try_into().unwrap_or_default(),
            self.green().try_into().unwrap_or_default(),
            self.blue().try_into().unwrap_or_default(),
        )
    }
}

pub trait GetTiming {
    fn timing(&self) -> RangeInclusive<i32>;
}

impl GetTiming for GrowthPrint {
    fn timing(&self) -> RangeInclusive<i32> {
        let start = if self.timing_start().is_negative() {
            i32::MIN
        } else {
            self.timing_start()
        };
        let end = if self.timing_end().is_negative() {
            i32::MAX
        } else {
            self.timing_end()
        };
        start..=end
    }
}

impl GetTiming for TreeGrowth {
    fn timing(&self) -> RangeInclusive<i32> {
        let start = if self.timing_start().is_negative() {
            i32::MIN
        } else {
            self.timing_start()
        };
        let end = if self.timing_end().is_negative() {
            i32::MAX
        } else {
            self.timing_end()
        };
        start..=end
    }
}

impl ConsoleColor for GrowthPrint {
    fn get_console_color(&self) -> Color {
        Color::from_i32(self.color()).unwrap_or(Color::COLOR_BLACK)
    }
}

impl RGBColor for Color {
    fn rgb(&self) -> palette::Srgb<u8> {
        match self {
            Color::COLOR_BLACK => named::BLACK,
            Color::COLOR_BLUE => named::BLUE,
            Color::COLOR_GREEN => named::GREEN,
            Color::COLOR_CYAN => named::CYAN,
            Color::COLOR_RED => named::RED,
            Color::COLOR_MAGENTA => named::DARKMAGENTA,
            Color::COLOR_BROWN => named::BROWN,
            Color::COLOR_GREY => named::GRAY,
            Color::COLOR_DARKGREY => named::DARKGRAY,
            Color::COLOR_LIGHTBLUE => named::LIGHTBLUE,
            Color::COLOR_LIGHTGREEN => named::LIGHTGREEN,
            Color::COLOR_LIGHTCYAN => named::LIGHTCYAN,
            Color::COLOR_LIGHTRED => named::PINK,
            Color::COLOR_LIGHTMAGENTA => named::MAGENTA,
            Color::COLOR_YELLOW => named::YELLOW,
            Color::COLOR_WHITE => named::WHITE,
        }
    }
}

#[easy_ext::ext(BasicMaterialInfoExt)]
pub impl BasicMaterialInfo {
    fn flag_names<'a>(&self, enums: &'a ListEnumsOut) -> Vec<&'a str> {
        self.flags
            .iter()
            .map(|flag| enums.material_flags[*flag as usize].name())
            .collect()
    }
}

#[easy_ext::ext(SpatterExt)]
pub impl Spatter {
    /// spatter proportion from 0 to one
    fn amount_normalized(&self) -> f32 {
        match self.state() {
            dfhack_remote::MatterState::Solid => self.amount() as f32 / 10000.0,
            dfhack_remote::MatterState::Liquid => self.amount() as f32 / 255.0,
            dfhack_remote::MatterState::Gas => 0.0,
            dfhack_remote::MatterState::Powder => self.amount() as f32 / 100.0,
            dfhack_remote::MatterState::Paste => 0.0,
            dfhack_remote::MatterState::Pressed => 0.0,
        }
        .clamp(0.0, 1.0)
    }
}

#[easy_ext::ext(BuildingExt)]
pub impl BuildingInstance {
    fn building_flags_typed(&self) -> BuildingFlags {
        BuildingFlags::from_bits_retain(self.building_flags())
    }
}

pub fn create_building_def_map(
    building_definitions: dfhack_remote::BuildingList,
) -> HashMap<(i32, i32, i32), BuildingDefinition> {
    let building_map: HashMap<(i32, i32, i32), BuildingDefinition> = building_definitions
        .building_list
        .into_iter()
        .map(|b| {
            let t = b.building_type.get_or_default();
            (
                (t.building_type(), t.building_subtype(), t.building_custom()),
                b,
            )
        })
        .collect();
    building_map
}
