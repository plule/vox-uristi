#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod building;
mod calendar;
mod context;
mod coords;
mod direction;
mod dot_vox_builder;
mod export;
mod flow;
mod map;
mod palette;
mod prefabs;
mod rfr;
mod shape;
mod tile;
mod traits;
mod ui;
#[cfg(feature = "self-update")]
mod update;
mod voxel;

use std::path::PathBuf;

use calendar::Month;
use export::Elevation;
pub use traits::*;

use clap::{Parser, Subcommand};
pub use coords::{
    DFBoundingBox, DFCoords, VoxelCoords, WithDFCoords, WithVoxelCoords, BASE, HEIGHT,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[cfg(feature = "gui")]
    #[command(subcommand)]
    pub command: Option<Command>,

    #[cfg(not(feature = "gui"))]
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run with a graphical user interface
    #[cfg(feature = "gui")]
    Gui,
    /// Export the map in the .vox format
    Export {
        /// Lower point to export
        #[arg(long, allow_hyphen_values = true)]
        low: Option<i32>,
        /// Higher point to export
        #[arg(long, allow_hyphen_values = true)]
        high: Option<i32>,
        /// Season for export
        #[arg(long)]
        month: Option<Month>,
        /// Destination file
        destination: PathBuf,
    },
    /// Export one .vox file per month of the year
    ExportYear {
        /// Lower point to export
        #[arg(long, allow_hyphen_values = true)]
        low: Option<i32>,
        /// Higher point to export
        #[arg(long, allow_hyphen_values = true)]
        high: Option<i32>,
        /// Destination folder
        destination: PathBuf,
    },
    /// Check for new versions
    #[cfg(feature = "self-update")]
    CheckUpdate,
    /// Developper utilities
    #[cfg(feature = "dev")]
    #[command(subcommand)]
    Dev(DevCommand),
}

#[derive(Subcommand)]
pub enum DevCommand {
    /// Regen test data from df
    RegenTestData,
    /// Debug the tile under the cursor
    Probe {
        /// Destination folder
        destination: PathBuf,
    },
    /// Dump the material, plant, raw lists...
    DumpLists {
        /// Destination folder
        destination: PathBuf,
    },
}

impl Cli {
    #[cfg(feature = "gui")]
    pub fn command(self) -> Command {
        self.command.unwrap_or(Command::Gui)
    }

    #[cfg(not(feature = "gui"))]
    pub fn command(self) -> Command {
        self.command
    }
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command() {
        #[cfg(feature = "gui")]
        Command::Gui => ui::gui::run(),
        Command::Export {
            low,
            high,
            destination,
            month,
        } => ui::cli::export(low.map(Elevation), high.map(Elevation), destination, month),
        Command::ExportYear {
            low,
            high,
            destination,
        } => ui::cli::export_year(low.map(Elevation), high.map(Elevation), destination),
        #[cfg(feature = "self-update")]
        Command::CheckUpdate => ui::cli::check_update(),
        #[cfg(feature = "dev")]
        Command::Dev(cmd) => ui::cli::dev::run(cmd),
    }
}
