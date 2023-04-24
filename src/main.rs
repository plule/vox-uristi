#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod cli;
pub mod export;
pub mod map;
pub mod palette;
pub mod tile;
pub mod rfr;
use app::App;
use clap::Parser;
use cli::{run_cli_command, Cli};
use eframe::egui;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon"));

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        run_cli_command(command)
    } else {
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
}
