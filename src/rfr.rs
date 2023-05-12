use crate::Coords;
use anyhow::Result;
use dfhack_remote::{
    core_text_fragment::Color, BlockList, BlockRequest, ColorDefinition, GrowthPrint, MapBlock,
    MatPair, Tiletype, TiletypeList, TreeGrowth,
};
use palette::{named, Srgb};
use protobuf::Enum;
use std::{
    fmt::{Debug, Display},
    ops::{Range, RangeInclusive},
};

/// Wrapper around dwarf fortress blocks to help access individual tile properties
#[derive(Debug)]
pub struct BlockTile<'a> {
    block: &'a MapBlock,
    index: usize,
    tiletypes: &'a TiletypeList,
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
                Some(Ok(blocks))
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
        }
    }

    pub fn coords(&self) -> Coords {
        let (sub_x, sub_y) = (self.index % 16, self.index / 16);
        Coords::new(
            self.block.map_x() + sub_x as i32,
            self.block.map_y() + sub_y as i32,
            self.block.map_z(),
        )
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

    pub fn tree(&self) -> Coords {
        Coords::new(
            self.block.tree_x[self.index],
            self.block.tree_y[self.index],
            self.block.tree_z[self.index],
        )
    }

    pub fn tree_origin(&self) -> Coords {
        let coord = self.coords();
        let tree = self.tree();
        Coords::new(coord.x - tree.x, coord.y - tree.y, coord.z + tree.z)
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
}

impl Display for BlockTile<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "coords: {}", self.coords())?;
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
