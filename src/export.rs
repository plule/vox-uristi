use crate::{
    dot_vox_builder::DotVoxBuilder, map::Map, palette::Palette, rfr, voxel::CollectVoxels,
};
use anyhow::Result;
use dfhack_remote::PlantRawList;
use dot_vox::DotVoxData;
use std::{
    fs::File,
    ops::Range,
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

pub struct ExportSettings {
    pub year_tick: i32,
}

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
    z_range: Range<i32>,
    year_tick: i32,
    path: PathBuf,
    progress_tx: Sender<Progress>,
    cancel_rx: Receiver<Cancel>,
) -> Result<()> {
    client.remote_fortress_reader().set_pause_state(true)?;
    client.remote_fortress_reader().reset_map_hashes()?;

    let tile_type_list = client.remote_fortress_reader().get_tiletype_list()?;
    let material_list = client.remote_fortress_reader().get_material_list()?;
    let map_info = client.remote_fortress_reader().get_map_info()?;
    let plant_raws = client.remote_fortress_reader().get_plant_raws()?;

    let block_list_iterator =
        rfr::BlockListIterator::try_new(client, 100, 0..1000, 0..1000, z_range.clone())?;
    let (block_list_count, _) = block_list_iterator.size_hint();

    let mut map = Map::default();
    progress_tx.send(Progress::StartReading {
        total: block_list_count,
    })?;
    let settings = ExportSettings { year_tick };

    let mut blocks = Vec::new();

    for (progress, block_list) in block_list_iterator.enumerate() {
        if cancel_rx.try_iter().next().is_some() {
            return Ok(());
        }

        progress_tx.send(Progress::Reading {
            curr: progress,
            to: block_list_count,
        })?;

        for block in block_list?.map_blocks {
            blocks.push(block);
        }
    }

    for block in blocks.iter() {
        map.add_block(block, &tile_type_list);
    }

    map.remove_overlapping_floors();

    let total = map.tiles.len();
    progress_tx.send(Progress::StartBuilding { total })?;

    let mut vox = DotVoxBuilder::default();
    let mut palette = Palette::default();

    let max_y = map_info.block_size_y() * 16 * 3;
    let min_z = z_range.start;

    for building_list in map.buildings.values() {
        for building in building_list {
            add_voxels(
                *building,
                &map,
                &settings,
                &plant_raws,
                &mut palette,
                &mut vox,
                max_y,
                min_z,
            );
        }
    }

    for (progress, tile) in map.tiles.values().enumerate() {
        if cancel_rx.try_iter().next().is_some() {
            return Ok(());
        }

        progress_tx.send(Progress::Building {
            curr: progress,
            to: total,
        })?;
        add_voxels(
            tile,
            &map,
            &settings,
            &plant_raws,
            &mut palette,
            &mut vox,
            max_y,
            min_z,
        );
    }

    for flow in map.flows.values() {
        add_voxels(
            flow,
            &map,
            &settings,
            &plant_raws,
            &mut palette,
            &mut vox,
            max_y,
            min_z,
        );
    }

    let mut vox: DotVoxData = vox.into();

    palette.write_palette(&mut vox, &material_list.material_list);
    progress_tx.send(Progress::Writing)?;
    let mut f = File::create(path.clone())?;
    vox.write_vox(&mut f)?;
    progress_tx.send(Progress::Done { path })?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_voxels<T>(
    item: &T,
    map: &Map,
    settings: &ExportSettings,
    plant_raws: &PlantRawList,
    palette: &mut Palette,
    vox: &mut DotVoxBuilder,
    max_y: i32,
    min_z: i32,
) where
    T: CollectVoxels,
{
    for voxel in item.collect_voxels(map, settings, plant_raws) {
        let color = palette.get_palette_color(&voxel.material);
        vox.add_voxel(
            voxel.coord.x,
            max_y - voxel.coord.y,
            voxel.coord.z - min_z,
            color,
        );
    }
}

pub fn export_voxels(
    client: &mut dfhack_remote::Client,
    elevation_range: Range<i32>,
    yeah_tick: i32,
    path: PathBuf,
    progress_tx: Sender<Progress>,
    cancel_rx: Receiver<Cancel>,
) {
    if let Err(err) = try_export_voxels(
        client,
        elevation_range,
        yeah_tick,
        path,
        progress_tx.clone(),
        cancel_rx,
    ) {
        progress_tx
            .send(Progress::Error(err))
            .expect("Failed to report error");
    }
}
