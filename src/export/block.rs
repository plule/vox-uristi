//! Functions to export voxels of a block (16x16x1 tiles chunk of the map)

use std::collections::HashMap;

use dfhack_remote::MapBlock;
use dot_vox::Model;
use itertools::Itertools;

use crate::{
    coords::DotVoxModelCoords,
    dot_vox_builder::{DotVoxBuilder, NodeId},
    rfr, WithDFCoords, BASE,
};

use super::{DFContext, FlowInfoExt, Layers, Map, Models, Palette, BLOCK_VOX_SIZE};

/// All the voxel models constituing a block
#[derive(Default)]
pub struct BlockModels {
    pub models: HashMap<Layers, Model>,
}

pub fn build(
    block: &MapBlock,
    map: &Map,
    context: &DFContext,
    vox: &mut DotVoxBuilder,
    palette: &mut Palette,
    level_group_id: NodeId,
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
            level_group_id,
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
            models.extend(Layers::Flows, flow.build(context, palette));
        }
    }

    if models.is_empty() {
        // Empty groups are shown as big cubes, skip
        return;
    }

    let block_group = vox.insert_group_node_simple(
        level_group_id,
        format!("block {} {}", block.map_x(), block.map_y(),),
        Some(DotVoxModelCoords::new(x, y, 0)),
        Layers::All.id(),
    );

    models.build(vox, block_group);
}

impl BlockModels {
    pub fn is_empty(&self) -> bool {
        self.models.values().all(|m| m.voxels.is_empty())
    }

    pub fn get(&mut self, layer: Layers) -> &mut Model {
        self.models
            .entry(layer)
            .or_insert_with(|| DotVoxBuilder::new_model(BLOCK_VOX_SIZE))
    }

    pub fn extend(&mut self, layer: Layers, voxels: impl IntoIterator<Item = dot_vox::Voxel>) {
        self.get(layer).voxels.extend(voxels);
    }

    pub fn build(self, vox: &mut DotVoxBuilder, group_id: NodeId) {
        for (layer, model) in self.models.into_iter().sorted_by_key(|(l, _)| *l).rev() {
            if model.voxels.is_empty() {
                continue;
            }
            vox.insert_model_and_shape_node(group_id, None, model, layer.id(), layer.to_string());
        }
    }
}
