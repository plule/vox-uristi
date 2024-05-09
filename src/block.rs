use dfhack_remote::MapBlock;
use dot_vox::{Model, Size};

use crate::{
    context::DFContext,
    coords::DotVoxModelCoords,
    dot_vox_builder::{DotVoxBuilder, NodeId},
    export::{Layers, Models},
    flow::FlowInfoExt,
    rfr, WithDFCoords, BASE, HEIGHT,
};

pub const BLOCK_SIZE: usize = 16;

pub const BLOCK_VOX_SIZE: Size = Size {
    x: (BLOCK_SIZE * BASE) as u32,
    y: (BLOCK_SIZE * BASE) as u32,
    z: HEIGHT as u32,
};

/// All the voxel models constituing a block
pub struct BlockModels {
    pub terrain: Model,
    pub liquid: Model,
    pub spatter: Model,
    pub fire: Model,
    pub flows: Model,
    pub hidden: Model,
}

pub fn build(
    block: &MapBlock,
    map: &crate::map::Map,
    context: &DFContext,
    vox: &mut DotVoxBuilder,
    palette: &mut crate::palette::Palette,
    layer_group_id: NodeId,
) {
    // Collect all the tiles of the block
    let tiles: Vec<_> = rfr::TileIterator::new(block, &context.tile_types).collect();

    if tiles.is_empty() {
        // The block is empty, skip the construction
        return;
    }

    // Create the parent group for all the objects of this block
    let x = block.map_x() * BASE as i32 - context.max_vox_x() + 24;
    let y = context.max_vox_y() - block.map_y() * BASE as i32 - 23;

    if tiles.iter().all(|t| t.hidden()) {
        // The full block is hidden, skip the construction and add the
        // hidden model to save space
        let block_group = vox.insert_group_node_simple(
            layer_group_id,
            format!("block {} {}", block.map_x(), block.map_y(),),
            Some(DotVoxModelCoords::new(x, y, 0)),
            Layers::All.id(),
        );
        vox.insert_shape_node_simple(
            block_group,
            "hidden",
            None,
            Layers::Hidden.id(),
            Models::HiddenBlock.id(),
        );
        return;
    }

    let mut models = BlockModels::default();

    for tile in tiles {
        tile.build(&mut models, map, context, palette);

        for flow in block
            .flows
            .iter()
            .filter(|flow| flow.coords() == tile.global_coords())
        {
            models.flows.voxels.extend(flow.build(context, palette));
        }
    }

    if models.is_empty() {
        // Empty groups are shown as big cubes, skip
        return;
    }

    let block_group = vox.insert_group_node_simple(
        layer_group_id,
        format!("block {} {}", block.map_x(), block.map_y(),),
        Some(DotVoxModelCoords::new(x, y, 0)),
        Layers::All.id(),
    );

    // Add the non empty models to the .vox
    // The order matters, the last added model will be on top of the others

    if !models.flows.voxels.is_empty() {
        vox.insert_model_and_shape_node(
            block_group,
            None,
            models.flows,
            Layers::Flows.id(),
            "flows",
        );
    }

    if !models.fire.voxels.is_empty() {
        vox.insert_model_and_shape_node(block_group, None, models.fire, Layers::Fire.id(), "fire");
    }

    if !models.liquid.voxels.is_empty() {
        vox.insert_model_and_shape_node(
            block_group,
            None,
            models.liquid,
            Layers::Liquid.id(),
            "liquid",
        );
    }

    if !models.spatter.voxels.is_empty() {
        vox.insert_model_and_shape_node(
            block_group,
            None,
            models.spatter,
            Layers::Spatter.id(),
            "spatter",
        );
    }

    if !models.hidden.voxels.is_empty() {
        vox.insert_model_and_shape_node(
            block_group,
            None,
            models.hidden,
            Layers::Hidden.id(),
            "hidden",
        );
    }

    if !models.terrain.voxels.is_empty() {
        vox.insert_model_and_shape_node(
            block_group,
            None,
            models.terrain,
            Layers::Terrain.id(),
            "terrain",
        );
    }
}

impl Default for BlockModels {
    fn default() -> Self {
        Self {
            terrain: DotVoxBuilder::new_model(BLOCK_VOX_SIZE),
            liquid: DotVoxBuilder::new_model(BLOCK_VOX_SIZE),
            spatter: DotVoxBuilder::new_model(BLOCK_VOX_SIZE),
            fire: DotVoxBuilder::new_model(BLOCK_VOX_SIZE),
            flows: DotVoxBuilder::new_model(BLOCK_VOX_SIZE),
            hidden: DotVoxBuilder::new_model(BLOCK_VOX_SIZE),
        }
    }
}

impl BlockModels {
    pub fn is_empty(&self) -> bool {
        self.terrain.voxels.is_empty()
            && self.liquid.voxels.is_empty()
            && self.spatter.voxels.is_empty()
            && self.fire.voxels.is_empty()
            && self.flows.voxels.is_empty()
            && self.hidden.voxels.is_empty()
    }
}
