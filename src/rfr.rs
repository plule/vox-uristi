use crate::map::Coords;
use anyhow::Result;
use dfhack_remote::{
    BlockList, BlockRequest, MapBlock, MatPair, MaterialDefinition, Tiletype, TiletypeList,
};
use std::{collections::HashMap, fmt::Debug, ops::Range};

/// Wrapper around dwarf fortress blocks to help access individual tile properties
#[derive(Debug)]
pub struct BlockTile<'a> {
    block: &'a MapBlock,
    index: usize,
    materials: &'a HashMap<MatPair, MaterialDefinition>,
    tiletypes: &'a TiletypeList,
}

pub struct BlockListIterator<'a> {
    client: &'a mut dfhack_remote::Stubs<dfhack_remote::Channel>,
    block_per_it: i32,
    x_range: Range<i32>,
    y_range: Range<i32>,
    z_range: Range<i32>,
    remaining: usize,
}

pub struct TileIterator<'a> {
    block: &'a MapBlock,
    index: usize,
    materials: &'a HashMap<MatPair, MaterialDefinition>,
    tiletypes: &'a TiletypeList,
}

#[allow(clippy::mutable_key_type)] // possibly an actual issue?
impl<'a> TileIterator<'a> {
    pub fn new(
        block: &'a MapBlock,
        materials: &'a HashMap<MatPair, MaterialDefinition>,
        tiletypes: &'a TiletypeList,
    ) -> Self {
        Self {
            block,
            materials,
            index: 0,
            tiletypes,
        }
    }
}

impl<'a> Iterator for TileIterator<'a> {
    type Item = BlockTile<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        if self.index <= self.block.tiles.len() {
            Some(BlockTile::new(
                self.block,
                self.index - 1,
                self.materials,
                self.tiletypes,
            ))
        } else {
            None
        }
    }
}

impl<'a> BlockListIterator<'a> {
    pub fn try_new(
        client: &'a mut dfhack_remote::Stubs<dfhack_remote::Channel>,
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

#[allow(clippy::mutable_key_type)] // possibly an actual issue?
impl<'a> BlockTile<'a> {
    pub fn new(
        block: &'a MapBlock,
        index: usize,
        materials: &'a HashMap<MatPair, MaterialDefinition>,
        tiletypes: &'a TiletypeList,
    ) -> Self {
        Self {
            block,
            index,
            materials,
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

    pub fn material_pair(&self) -> &MatPair {
        &self.block.materials[self.index]
    }

    pub fn material(&self) -> Option<&MaterialDefinition> {
        self.materials.get(self.material_pair())
    }

    pub fn base_material_pair(&self) -> &MatPair {
        &self.block.base_materials[self.index]
    }

    pub fn base_material(&self) -> Option<&MaterialDefinition> {
        self.materials.get(self.base_material_pair())
    }

    pub fn vein_material_pair(&self) -> &MatPair {
        &self.block.vein_materials[self.index]
    }

    pub fn vein_material(&self) -> Option<&MaterialDefinition> {
        self.materials.get(self.vein_material_pair())
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

pub fn build_material_map(
    client: &mut dfhack_remote::Stubs<dfhack_remote::Channel>,
) -> Result<HashMap<MatPair, MaterialDefinition>> {
    let materials = client.remote_fortress_reader().get_material_list()?;
    Ok(materials
        .material_list
        .into_iter()
        .map(|mat| (mat.mat_pair.get_or_default().to_owned(), mat))
        .collect())
}
