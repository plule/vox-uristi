use crate::map::Coords;
use anyhow::{Context, Result};
use dfhack_remote::{BlockRequest, BuildingInstance, MaterialDefinition, Tiletype};
use generator::{done, Gn};
use std::{collections::HashMap, ops::Range};

#[derive(Debug)]
pub struct DFTile<'a> {
    pub coords: Coords,
    pub hidden: bool,
    pub water: i32,
    pub tile_type: &'a Tiletype,
    pub material: Option<&'a MaterialDefinition>,
    pub base_material: Option<&'a MaterialDefinition>,
    pub vein_material: Option<&'a MaterialDefinition>,
    pub magma: i32,
    pub water_stagnant: bool,
    pub water_salt: bool,
    pub tree: Coords,
    pub tree_origin: Coords,
    pub tree_percent: i32,
    pub grass_percent: Option<i32>,
}

pub fn iter_tiles<'a>(
    client: &'a mut dfhack_remote::Stubs<dfhack_remote::Channel>,
    block_per_it: i32,
    x_range: Range<i32>,
    y_range: Range<i32>,
    z_range: Range<i32>,
    tile_types: &'a [Tiletype],
    materials: &'a [MaterialDefinition],
) -> Result<(usize, impl Iterator<Item = Result<DFTile<'a>>>)> {
    let map_info = client.remote_fortress_reader().get_map_info()?;
    let size_x = map_info.block_size_x() as usize;
    let size_y = map_info.block_size_y() as usize;
    let size_z = (z_range.end - z_range.start) as usize;
    let tile_number = size_x * size_y * size_z * 256;

    Ok((
        tile_number,
        Gn::new_scoped_opt_local(4096 * 4, move |mut s| {
            client.remote_fortress_reader().reset_map_hashes()?;
            #[allow(clippy::mutable_key_type)] // possibly an actual issue?
            let mut material_map = HashMap::new();
            for material in materials.iter() {
                material_map.insert(material.mat_pair.clone().unwrap_or_default(), material);
            }

            loop {
                let mut req = BlockRequest::new();
                req.set_blocks_needed(block_per_it);
                req.set_min_x(x_range.start);
                req.set_max_x(x_range.end);
                req.set_min_y(y_range.start);
                req.set_max_y(y_range.end);
                req.set_min_z(z_range.start);
                req.set_max_z(z_range.end);
                let blocks = client.remote_fortress_reader().get_block_list(req)?;
                let mut block_count = 0;
                for block in blocks.map_blocks.iter().filter(|b| b.tiles.len() == 256) {
                    block_count += 1;
                    let hiddens = &block.hidden;
                    let tile_types_indexes = &block.tiles;
                    let materials = &block.materials;
                    let base_materials = &block.base_materials;
                    let vein_materials = &block.vein_materials;
                    let waters = &block.water;
                    let magmas = &block.magma;
                    let water_stagnants = &block.water_stagnant;
                    let water_salts = &block.water_salt;
                    let tree_percent = &block.tree_percent;
                    let grass_percents = &block.grass_percent;
                    let tree_x = &block.tree_x;
                    let tree_y = &block.tree_y;
                    let tree_z = &block.tree_z;

                    let map_x = block.map_x();
                    let map_y = block.map_y();
                    let z = block.map_z();

                    for sub_x in 0..16 {
                        for sub_y in 0..16 {
                            let index = (sub_y * 16 + sub_x) as usize;
                            let x = map_x + sub_x;
                            let y = map_y + sub_y;
                            let matpairs = &materials[index];
                            let base_batpairs = &base_materials[index];
                            let vein_batpairs = &vein_materials[index];
                            s.yield_(Ok(DFTile {
                                coords: Coords { x, y, z },
                                hidden: hiddens[index],
                                water: waters[index],
                                tile_type: &tile_types[tile_types_indexes[index] as usize],
                                material: material_map.get(matpairs).copied(),
                                base_material: material_map.get(base_batpairs).copied(),
                                vein_material: material_map.get(vein_batpairs).copied(),
                                magma: magmas[index],
                                water_stagnant: water_stagnants[index],
                                water_salt: water_salts[index],
                                tree: Coords::new(tree_x[index], tree_y[index], tree_z[index]),
                                tree_origin: Coords::new(
                                    x - tree_x[index],
                                    y - tree_y[index],
                                    z + tree_z[index],
                                ),
                                tree_percent: tree_percent[index],
                                grass_percent: grass_percents.get(index).copied(),
                            }));
                        }
                    }
                }

                if block_count == 0 {
                    done!()
                }
            }
        }),
    ))
}

pub fn iter_buildings(
    client: &mut dfhack_remote::Stubs<dfhack_remote::Channel>,
    x_range: Range<i32>,
    y_range: Range<i32>,
    z_range: Range<i32>,
) -> Result<(usize, impl Iterator<Item = BuildingInstance>)> {
    client.remote_fortress_reader().reset_map_hashes()?;
    let mut req = BlockRequest::new();
    req.set_blocks_needed(100);
    req.set_min_x(x_range.start);
    req.set_max_x(x_range.end);
    req.set_min_y(y_range.start);
    req.set_max_y(y_range.end);
    req.set_min_z(z_range.start);
    req.set_max_z(z_range.end);
    let blocks = client.remote_fortress_reader().get_block_list(req)?;
    // items are only inserted on the first block
    let first_block = blocks.map_blocks.first().context("No block")?.clone();
    let item_number = first_block.buildings.len();

    Ok((
        item_number,
        Gn::new_scoped_opt_local(4096 * 4, move |mut s| {
            for building in first_block.buildings {
                s.yield_(building);
            }
            done!()
        }),
    ))
}
