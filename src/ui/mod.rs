use std::{
    fmt::Display,
    ops::{Add, Sub},
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    calendar::TimeOfTheYear,
    export::{Cancel, Progress},
    update::UpdateStatus,
};

/// Command line interface
pub mod cli;
/// Graphical user interface
pub mod gui;
/// Text user interface
pub mod tui;

/// Serializable application state, shared between the GUI and the TUI
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
    update_status: CheckUpdateStatus,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
struct Elevation(pub i32);

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

enum CheckUpdateStatus {
    NotDone,
    Doing(Receiver<Result<UpdateStatus>>),
    Done(UpdateStatus),
}

/// Ability to be read from dwarf fortress
trait FromDwarfFortress {
    fn read_from_df(
        &mut self,
        df: &mut dfhack_remote::Result<dfhack_remote::Client>,
    ) -> Result<()> {
        if df.is_err() {
            *df = dfhack_remote::connect();
        }
        if let Ok(df) = df {
            self.do_read_from_df(df)?;
        }
        Ok(())
    }

    fn do_read_from_df(&mut self, df: &mut dfhack_remote::Client) -> Result<()>;
}

impl Default for State {
    fn default() -> Self {
        Self {
            low_elevation: Elevation(100),
            high_elevation: Elevation(110),
            time: Default::default(),
            error: Default::default(),
            progress: Default::default(),
            exported_path: Default::default(),
            update_status: Default::default(),
        }
    }
}

impl Default for CheckUpdateStatus {
    fn default() -> Self {
        Self::NotDone
    }
}

impl FromDwarfFortress for Elevation {
    fn do_read_from_df(&mut self, df: &mut dfhack_remote::Client) -> Result<()> {
        self.0 = df.remote_fortress_reader().get_view_info()?.view_pos_z();
        Ok(())
    }
}

impl FromDwarfFortress for TimeOfTheYear {
    fn do_read_from_df(&mut self, _df: &mut dfhack_remote::Client) -> Result<()> {
        // todo: refine for better display
        *self = TimeOfTheYear::Current;
        Ok(())
    }
}
