#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod building;
mod calendar;
#[cfg(feature = "cli")]
mod cli;
mod direction;
mod export;
mod flow;
mod map;
mod palette;
mod rfr;
mod shape;
mod tile;
mod update;
mod voxel;
use std::fmt::Display;

use app::App;
use eframe::egui;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon"));

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Coords {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Display for Coords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}

pub trait IsSomeAnd<T> {
    fn some_and(self, f: impl FnOnce(T) -> bool) -> bool;
}

impl<T> IsSomeAnd<T> for Option<T> {
    fn some_and(self, f: impl FnOnce(T) -> bool) -> bool {
        match self {
            None => false,
            Some(x) => f(x),
        }
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "cli")]
    {
        use clap::Parser;
        let cli = cli::Cli::parse();

        if let Some(command) = cli.command {
            return cli::run_cli_command(command);
        }
    }
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 320.0)),
        icon_data: Some(eframe::IconData {
            rgba: ICON.to_vec(),
            width: 256,
            height: 256,
        }),
        ..Default::default()
    };
    match eframe::run_native(
        format!("Vox Uristi v{VERSION}").as_str(),
        options,
        Box::new(|cc| Box::<App>::new(app::App::new(cc))),
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::format_err!("{}", e.to_string())),
    }
}
