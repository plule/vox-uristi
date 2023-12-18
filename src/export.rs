use crate::{
    calendar::TimeOfTheYear,
    context::DFContext,
    dot_vox_builder::DotVoxBuilder,
    map::Map,
    palette::Palette,
    rfr::{self, DFHackExt},
    voxel::CollectVoxels,
    FromDwarfFortress, BASE, HEIGHT,
};
use anyhow::Result;
use dot_vox::DotVoxData;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::File,
    ops::{Add, Range, Sub},
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
    thread::JoinHandle,
};

pub struct ExportParams {
    pub elevation_low: Elevation,
    pub elevation_high: Elevation,
    pub time: TimeOfTheYear,
    pub path: PathBuf,
}

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

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Elevation(pub i32);

impl FromDwarfFortress for Elevation {
    fn read_from_df(&mut self, df: &mut dfhack_remote::Client) -> Result<()> {
        self.0 = df.elevation()?;
        Ok(())
    }
}

impl Add<i32> for Elevation {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<i32> for Elevation {
    type Output = Self;

    fn sub(self, rhs: i32) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Display for Elevation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn try_export_voxels(
    client: &mut dfhack_remote::Client,
    elevation_range: Range<Elevation>,
    year_tick: i32,
    path: PathBuf,
    progress_tx: Sender<Progress>,
    cancel_rx: Receiver<Cancel>,
) -> Result<()> {
    progress_tx.send(Progress::undetermined("Starting..."))?;
    client.remote_fortress_reader().set_pause_state(true)?;
    client.remote_fortress_reader().reset_map_hashes()?;
    let z_offset = client.elevation_offset()?;
    let z_range = (elevation_range.start.0 - z_offset)..(elevation_range.end.0 - z_offset);
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

    let total = map.tiles.len();
    progress_tx.send(Progress::start("Building tiles...", total))?;
    for (curr, tile) in map.tiles.values().enumerate() {
        if cancel_rx.try_iter().next().is_some() {
            return Ok(());
        }

        progress_tx.send(Progress::update("Building tiles...", curr, total))?;

        for building in &tile.buildings {
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

        if let Some(df_tile) = &tile.block_tile {
            add_voxels(
                df_tile,
                &map,
                &context,
                &mut palette,
                &mut vox,
                max_x,
                max_y,
                min_z,
            );
        }

        for flow in &tile.flows {
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

pub fn try_run_export(
    params: ExportParams,
    df: Option<dfhack_remote::Client>,
    progress_tx: Sender<Progress>,
    cancel_rx: Receiver<Cancel>,
) -> Result<()> {
    let mut df = match df {
        Some(df) => df,
        None => dfhack_remote::connect()?,
    };

    let ticks = params.time.ticks(&mut df);

    try_export_voxels(
        &mut df,
        params.elevation_low..(params.elevation_high + 1),
        ticks,
        params.path,
        progress_tx,
        cancel_rx,
    )?;

    Ok(())
}

/// Run the export in a background thread, returns progress and cancellation channels
pub fn run_export_thread(
    params: ExportParams,
    df: Option<dfhack_remote::Client>,
) -> (Receiver<Progress>, Sender<Cancel>, JoinHandle<()>) {
    let (progress_tx, progress_rx) = std::sync::mpsc::channel();
    let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();

    let handle = std::thread::spawn(move || {
        if let Err(err) = try_run_export(params, df, progress_tx.clone(), cancel_rx) {
            // eat send error
            let _ = progress_tx.send(Progress::error(err));
        }
    });

    (progress_rx, cancel_tx, handle)
}
