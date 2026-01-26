//! Entrypoint function for the voxel export

mod block;
mod building;
mod context;
mod flow;
mod map;
mod palette;
mod prefabs;
mod tile;

pub use context::DFContext;
pub use flow::FlowInfoExt;
pub use map::Map;
pub use palette::{DefaultMaterials, EffectiveMaterial, Material, Palette};
pub use prefabs::{FromPrefab, MODELS};
pub use tile::BlockTileExt;

use crate::{
    calendar::TimeOfTheYear,
    coords::DotVoxModelCoords,
    dot_vox_builder::{DotVoxBuilder, LayerId, ModelId},
    rfr::{self, DFHackExt},
    FromDwarfFortress, BASE, HEIGHT,
};
use anyhow::Result;
use building::BuildingInstanceExt;
use dot_vox::{DotVoxData, Model, Size};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::File,
    ops::{Add, Range, Sub},
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
    thread::JoinHandle,
};
use strum::{Display, EnumIter, IntoEnumIterator};

/// Width and height of a Dwarf Fortress block
pub const BLOCK_SIZE: usize = 16;

/// Dimension in voxels of a Dwarf Fortress block
pub const BLOCK_VOX_SIZE: Size = Size {
    x: (BLOCK_SIZE * BASE) as u32,
    y: (BLOCK_SIZE * BASE) as u32,
    z: HEIGHT as u32,
};

/// List of displayed layers
/// The order is important, when building objects they are created in reverse order
/// As a result, each layer is rendered on top of the next one
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, EnumIter, Display)]
#[repr(usize)]
pub enum Layers {
    All,
    Building,
    Liquid,
    Terrain,
    Vegetation,
    Spatter,
    Fire,
    Flows,
    Hidden,
}

#[derive(Debug, Clone, Copy, EnumIter, Display)]
#[repr(usize)]
pub enum Models {
    HiddenBlock,
}

impl Layers {
    pub fn id(&self) -> LayerId {
        LayerId(*self as usize)
    }
}

impl Models {
    pub fn id(&self) -> ModelId {
        ModelId(*self as usize)
    }
}

/// Parameters for the export
pub struct ExportParams {
    /// Lower elevation limit
    pub elevation_low: Elevation,
    /// Upper elevation limit
    pub elevation_high: Elevation,
    /// Time of the year for tree growth
    pub time: TimeOfTheYear,
    /// Destination file
    pub path: PathBuf,
}

/// Settings influencing the export data
pub struct ExportSettings {
    /// Year tick, based on the selected time of the year, for tree growth
    pub year_tick: i32,
}

/// Progress status for UIs
pub enum Progress {
    /// Start a step without progress
    Undetermined { message: &'static str },
    /// Start a step with progress
    Start { message: &'static str, total: usize },
    /// Update a started step
    Update {
        message: &'static str,
        curr: usize,
        total: usize,
    },
    /// Finished
    Done { path: PathBuf },
    /// Failed
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
        Self::Update {
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
        let block_list = block_list?;

        for block in block_list.map_blocks {
            blocks.push(block);
        }

        for engraving in block_list.engravings {
            map.add_engraving(engraving);
        }
    }

    let tot = blocks.len();
    progress_tx.send(Progress::start("Assembling...", tot))?;
    for (curr, block) in blocks.iter().enumerate() {
        progress_tx.send(Progress::update("Assembling...", curr, tot))?;
        map.add_block(block, &context);
    }

    if context.adventure() {
        progress_tx.send(Progress::undetermined("Recomputing hidden blocks"))?;
        map.recompute_hidden();
    }

    progress_tx.send(Progress::undetermined("Cleaning..."))?;

    // Setup the palette, with the default material pre-inserted
    // to be easily findable
    let mut palette = Palette::default();
    palette.cache_default_materials(&context);

    let mut vox = DotVoxBuilder::default();
    vox.data
        .models
        .resize_with(Models::iter().count(), || Model {
            size: Size { x: 0, y: 0, z: 0 },
            voxels: vec![],
        });
    // Setup the default models
    {
        vox.data.models[*Models::HiddenBlock.id()].size = BLOCK_VOX_SIZE;
        for x in 0..BLOCK_VOX_SIZE.x {
            for y in 0..BLOCK_VOX_SIZE.y {
                for z in 0..BLOCK_VOX_SIZE.z {
                    vox.data.models[*Models::HiddenBlock.id()]
                        .voxels
                        .push(dot_vox::Voxel {
                            x: x as u8,
                            y: y as u8,
                            z: z as u8,
                            i: palette.get(&Material::Default(DefaultMaterials::Hidden), &context),
                        });
                }
            }
        }
    }

    // Setup the layers
    for layer in Layers::iter() {
        vox.data.layers[*layer.id()]
            .attributes
            .insert("_name".to_string(), format!("{}", layer).to_lowercase());
    }
    vox.data.layers[*Layers::Hidden.id()]
        .attributes
        .insert("_hidden".to_string(), "1".to_string());

    let min_z = z_range.start * HEIGHT as i32;
    let block_count = map.levels.values().map(|l| l.blocks.len()).sum();
    progress_tx.send(Progress::start("Building blocks...", block_count))?;
    let mut progress = 0;

    for (level, level_data) in map.levels.iter().sorted_by_key(|(l, _)| *l) {
        // Create a group for the layer
        let z = HEIGHT as i32 / 2 + level * HEIGHT as i32 - min_z;
        let level_group = vox.insert_group_node_simple(
            vox.root_group,
            format!("level {}", level + z_offset),
            Some(DotVoxModelCoords::new(0, 0, z)),
            Layers::All.id(),
        );

        for block in &level_data.blocks {
            progress += 1;
            progress_tx.send(Progress::update(
                "Building blocks...",
                progress,
                block_count,
            ))?;
            if cancel_rx.try_iter().next().is_some() {
                return Ok(());
            }

            // Create the terrain model
            block::build(block, &map, &context, &mut vox, &mut palette, level_group);
        }

        if !level_data.buildings.is_empty() {
            let building_group_id =
                vox.insert_group_node_simple(level_group, "buildings", None, Layers::Building.id());
            for building in &level_data.buildings {
                building.build(&map, &context, &mut vox, &mut palette, building_group_id);
            }
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
