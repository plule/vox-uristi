#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod building;
mod calendar;
#[cfg(feature = "cli")]
mod cli;
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
mod update;
mod voxel;
use app::App;
pub use coords::{
    DFBoundingBox, DFCoords, VoxelCoords, WithDFCoords, WithVoxelCoords, BASE, HEIGHT,
};
use eframe::egui;
use rand::Rng;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon"));

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

pub trait GenBoolSafe: Rng {
    fn gen_bool_safe(&mut self, probability: f64) -> bool {
        self.gen_bool(probability.clamp(0.0, 1.0))
    }
}

impl<T: Rng> GenBoolSafe for T {}

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
