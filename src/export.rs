use crate::{map::Map, palette::Palette, rfr};
use anyhow::Result;
use std::{
    ops::Range,
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

pub enum Progress {
    Connecting,
    StartReading { total: usize },
    Reading { curr: usize, to: usize },
    StartBuilding { total: usize },
    Building { curr: usize, to: usize },
    Writing,
    Done { path: PathBuf },
    Error(anyhow::Error),
}

pub struct Cancel;

pub fn try_export_voxels(
    client: &mut dfhack_remote::Client,
    elevation_range: Range<i32>,
    path: PathBuf,
    progress_tx: Sender<Progress>,
    cancel_rx: Receiver<Cancel>,
) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();
    client.remote_fortress_reader().set_pause_state(true)?;
    client.remote_fortress_reader().reset_map_hashes()?;

    let tile_type_list = client.remote_fortress_reader().get_tiletype_list()?;
    let material_list = client.remote_fortress_reader().get_material_list()?;
    let map_info = client.remote_fortress_reader().get_map_info()?;
    #[allow(clippy::mutable_key_type)] // possibly an actual issue?
    let material_map = rfr::build_material_map(client)?;

    let block_list_iterator =
        rfr::BlockListIterator::try_new(client, 100, 0..1000, 0..1000, elevation_range.clone())?;
    let (block_list_count, _) = block_list_iterator.size_hint();

    let mut map = Map::new();
    progress_tx.send(Progress::StartReading {
        total: block_list_count,
    })?;

    for (progress, block_list) in block_list_iterator.enumerate() {
        if cancel_rx.try_iter().next().is_some() {
            return Ok(());
        }

        progress_tx.send(Progress::Reading {
            curr: progress,
            to: block_list_count,
        })?;

        for block in block_list?.map_blocks {
            for tile in rfr::TileIterator::new(&block, &material_map, &tile_type_list) {
                map.add_tile(&tile);
            }

            for building in block.buildings {
                map.add_building(building);
            }
        }
    }

    let total = map.tiles.len();
    progress_tx.send(Progress::StartBuilding { total })?;

    let mut vox = vox_writer::VoxWriter::create_empty();
    let mut palette = Palette::default();
    palette.build_palette(
        map.tiles.values().map(|tile| &tile.material).chain(
            map.buildings
                .values()
                .flat_map(|v| v.iter().map(|b| &b.material)),
        ),
    );
    palette.write_palette(&mut vox, &material_list.material_list);

    let max_y = map_info.block_size_y() * 16 * 3;
    let min_z = elevation_range.start;

    for (progress, tile) in map.tiles.values().enumerate() {
        if cancel_rx.try_iter().next().is_some() {
            return Ok(());
        }

        progress_tx.send(Progress::Building {
            curr: progress,
            to: total,
        })?;
        let voxels = tile.collect_voxels(&palette, &map);
        for (coord, color) in voxels {
            vox.add_voxel(coord.x, max_y - coord.y, coord.z - min_z, color.into());
        }
    }

    for building_list in map.buildings.values() {
        for building in building_list {
            let voxels = building.collect_voxels(&palette, &map);
            for (coord, color) in voxels {
                vox.add_voxel(coord.x, max_y - coord.y, coord.z - min_z, color.into());
            }
        }
    }

    progress_tx.send(Progress::Writing)?;
    vox.save_to_file(path_str).expect("Fail to save vox file");
    progress_tx.send(Progress::Done { path })?;
    Ok(())
}

pub fn export_voxels(
    client: &mut dfhack_remote::Client,
    elevation_range: Range<i32>,
    path: PathBuf,
    progress_tx: Sender<Progress>,
    cancel_rx: Receiver<Cancel>,
) {
    if let Err(err) = try_export_voxels(
        client,
        elevation_range,
        path,
        progress_tx.clone(),
        cancel_rx,
    ) {
        progress_tx
            .send(Progress::Error(err))
            .expect("Failed to report error");
    }
}
