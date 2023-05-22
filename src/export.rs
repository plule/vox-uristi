use crate::{
    context::DFContext, dot_vox_builder::DotVoxBuilder, map::Map, palette::Palette, rfr,
    voxel::CollectVoxels, BASE, HEIGHT,
};
use anyhow::Result;
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
    Undetermined {
        message: &'static str,
    },
    Start {
        message: &'static str,
        total: usize,
    },
    Progress {
        message: &'static str,
        curr: usize,
        total: usize,
    },
    Done {
        path: PathBuf,
    },
    Error(anyhow::Error),
}

impl Progress {
    pub fn undetermined(message: &'static str) -> Self {
        Self::Undetermined { message }
    }

    pub fn start(message: &'static str, total: usize) -> Self {
        Self::Start { message, total }
    }

    pub fn update(message: &'static str, curr: usize, total: usize) -> Self {
        Self::Progress {
            message,
            curr,
            total,
        }
    }

    pub fn done(path: PathBuf) -> Self {
        Self::Done { path }
    }

    pub fn error(error: anyhow::Error) -> Self {
        Self::Error(error)
    }
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
    progress_tx.send(Progress::undetermined("Starting..."))?;
    client.remote_fortress_reader().set_pause_state(true)?;
    client.remote_fortress_reader().reset_map_hashes()?;
    let settings = ExportSettings { year_tick };
    let context = DFContext::try_new(client, settings)?;
    let block_list_iterator =
        rfr::BlockListIterator::try_new(client, 100, 0..1000, 0..1000, z_range.clone())?;
    let (block_list_count, _) = block_list_iterator.size_hint();

    let mut map = Map::default();

    let mut blocks = Vec::new();

    progress_tx.send(Progress::start("Reading...", block_list_count))?;
    for (progress, block_list) in block_list_iterator.enumerate() {
        if cancel_rx.try_iter().next().is_some() {
            return Ok(());
        }

        progress_tx.send(Progress::update("Reading...", progress, block_list_count))?;

        for block in block_list?.map_blocks {
            blocks.push(block);
        }
    }

    let tot = blocks.len();
    progress_tx.send(Progress::start("Assembling...", tot))?;
    for (curr, block) in blocks.iter().enumerate() {
        progress_tx.send(Progress::update("Assembling...", curr, tot))?;
        map.add_block(block, &context);
    }

    progress_tx.send(Progress::undetermined("Cleaning..."))?;
    map.remove_overlapping_floors(&context);

    let mut vox = DotVoxBuilder::default();
    let mut palette = Palette::default();

    let max_x = (context.map_info.block_size_x() * 16 * BASE as i32) / 2;
    let max_y = (context.map_info.block_size_y() * 16 * BASE as i32) / 2;
    let min_z = z_range.start * HEIGHT as i32;

    let total = map.buildings.len();
    progress_tx.send(Progress::start("Building constructions...", total))?;
    for (curr, building_list) in map.buildings.values().enumerate() {
        progress_tx.send(Progress::update("Building constructions...", curr, total))?;
        for building in building_list {
            add_voxels(
                *building,
                &map,
                &context,
                &mut palette,
                &mut vox,
                max_x,
                max_y,
                min_z,
            );
        }
    }

    let total = map.tiles.len();
    progress_tx.send(Progress::start("Building tiles...", total))?;
    for (curr, tile) in map.tiles.values().enumerate() {
        if cancel_rx.try_iter().next().is_some() {
            return Ok(());
        }

        progress_tx.send(Progress::update("Building tiles...", curr, total))?;
        add_voxels(
            tile,
            &map,
            &context,
            &mut palette,
            &mut vox,
            max_x,
            max_y,
            min_z,
        );
    }

    let total = map.flows.len();
    progress_tx.send(Progress::start("Building flows...", total))?;
    for (curr, flow) in map.flows.values().enumerate() {
        progress_tx.send(Progress::update("Building flows...", curr, total))?;
        add_voxels(
            flow,
            &map,
            &context,
            &mut palette,
            &mut vox,
            max_x,
            max_y,
            min_z,
        );
    }
    let mut vox: DotVoxData = vox.into();

    progress_tx.send(Progress::undetermined("Writing the palette..."))?;
    palette.write_palette(&mut vox);
    progress_tx.send(Progress::undetermined("Saving the file..."))?;
    let mut f = File::create(path.clone())?;
    vox.write_vox(&mut f)?;
    progress_tx.send(Progress::done(path))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_voxels<T>(
    item: &T,
    map: &Map,
    context: &DFContext,
    palette: &mut Palette,
    vox: &mut DotVoxBuilder,
    max_x: i32,
    max_y: i32,
    min_z: i32,
) where
    T: CollectVoxels,
{
    for voxel in item.collect_voxels(map, context) {
        let color = palette.get_palette_color(&voxel.material, context);
        vox.add_voxel(
            voxel.coord.x - max_x,
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
            .send(Progress::error(err))
            .expect("Failed to report error");
    }
}
