//! User interfaces, text and graphical
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    calendar::TimeOfTheYear,
    export::{Cancel, Elevation, ExportParams, Progress},
    FromDwarfFortress,
};

/// Command line interface
pub mod cli;
/// Graphical user interface
#[cfg(feature = "gui")]
pub mod gui;

/// Serializable application state
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    low_elevation: Elevation,
    high_elevation: Elevation,
    time: TimeOfTheYear,

    #[serde(skip)]
    error: Option<String>,
    #[serde(skip)]
    progress: Option<(Progress, Receiver<Progress>, Sender<Cancel>)>,
    #[serde(skip)]
    exported_path: Option<PathBuf>,
    #[serde(skip)]
    #[cfg(feature = "self-update")]
    update_status: CheckUpdateStatus,
}

#[cfg(feature = "self-update")]
#[derive(Default)]
enum CheckUpdateStatus {
    #[default]
    NotDone,
    Doing(Receiver<Result<crate::update::UpdateStatus>>),
    Done(crate::update::UpdateStatus),
}

impl Default for State {
    fn default() -> Self {
        Self {
            low_elevation: Elevation(0),
            high_elevation: Elevation(10),
            time: Default::default(),
            error: Default::default(),
            progress: Default::default(),
            exported_path: Default::default(),
            #[cfg(feature = "self-update")]
            update_status: Default::default(),
        }
    }
}

impl State {
    fn export_params(&self, path: PathBuf) -> ExportParams {
        ExportParams {
            elevation_low: self.low_elevation,
            elevation_high: self.high_elevation,
            time: self.time,
            path,
        }
    }
}

impl FromDwarfFortress for TimeOfTheYear {
    fn read_from_df(&mut self, _df: &mut dfhack_remote::Client) -> Result<()> {
        // todo: refine for better display
        *self = TimeOfTheYear::Current;
        Ok(())
    }
}
