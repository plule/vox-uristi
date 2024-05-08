use dfhack_remote::MapBlock;
use dot_vox::Size;

use crate::{
    context::DFContext,
    coords::DotVoxModelCoords,
    dot_vox_builder::{DotVoxBuilder, LayerId, NodeId},
    export::{FIRE_LAYER, FLOWS_LAYER, LIQUID_LAYER, SPATTER_LAYER, TERRAIN_LAYER, VOID_LAYER},
    flow::FlowInfoExt,
    rfr, WithDFCoords, BASE, HEIGHT,
};

pub const BLOCK_SIZE: usize = 16;

const BLOCK_VOX_SIZE: Size = Size {
    x: (BLOCK_SIZE * BASE) as u32,
    y: (BLOCK_SIZE * BASE) as u32,
    z: HEIGHT as u32,
};

pub fn build(
    block: &MapBlock,
    map: &crate::map::Map,
    context: &DFContext,
    vox: &mut DotVoxBuilder,
    palette: &mut crate::palette::Palette,
    layer_group_id: NodeId,
) {
    // Create the parent group for all the objects of this block
    let x = block.map_x() * BASE as i32 - context.max_vox_x() + 24;
    let y = context.max_vox_y() - block.map_y() * BASE as i32 - 23;

    let block_group = vox.insert_group_node_simple(
        layer_group_id,
        format!("block {} {}", block.map_x(), block.map_y(),),
        Some(DotVoxModelCoords::new(x, y, 0)),
        LayerId(0),
    );

    let mut terrain_model = DotVoxBuilder::new_model(BLOCK_VOX_SIZE);
    let mut liquid_model = DotVoxBuilder::new_model(BLOCK_VOX_SIZE);
    let mut spatter_model = DotVoxBuilder::new_model(BLOCK_VOX_SIZE);
    let mut fire_model = DotVoxBuilder::new_model(BLOCK_VOX_SIZE);
    let mut void_model = DotVoxBuilder::new_model(BLOCK_VOX_SIZE);
    let mut flows_model = DotVoxBuilder::new_model(BLOCK_VOX_SIZE);

    for tile in rfr::TileIterator::new(block, &context.tile_types) {
        let voxels = tile.build(map, context, palette);
        terrain_model.voxels.extend(voxels.terrain);
        liquid_model.voxels.extend(voxels.liquid);
        spatter_model.voxels.extend(voxels.spatter);
        fire_model.voxels.extend(voxels.fire);
        void_model.voxels.extend(voxels.void);

        for flow in block
            .flows
            .iter()
            .filter(|flow| flow.coords() == tile.global_coords())
        {
            flows_model.voxels.extend(flow.build(context, palette));
        }
    }

    // Add the non empty models to the .vox
    // The order matters, the last added model will be on top of the others

    if !flows_model.voxels.is_empty() {
        vox.insert_model_shape(block_group, None, flows_model, FLOWS_LAYER, "flows");
    }

    if !fire_model.voxels.is_empty() {
        vox.insert_model_shape(block_group, None, fire_model, FIRE_LAYER, "fire");
    }

    if !liquid_model.voxels.is_empty() {
        vox.insert_model_shape(block_group, None, liquid_model, LIQUID_LAYER, "liquid");
    }

    if !spatter_model.voxels.is_empty() {
        vox.insert_model_shape(block_group, None, spatter_model, SPATTER_LAYER, "spatter");
    }

    if !void_model.voxels.is_empty() {
        vox.insert_model_shape(block_group, None, void_model, VOID_LAYER, "void");
    }

    // The terrain itself is always added, to avoid weird sizing in MagicaVoxel with empty groups
    vox.insert_model_shape(block_group, None, terrain_model, TERRAIN_LAYER, "terrain");
}
