use crate::{export, tile_iterator};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::{path::PathBuf, thread};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    Export {
        /// Lower elevation bound to export
        elevation_low: i32,
        /// Higher elevation bound to export
        elevation_high: i32,
        /// Destination file
        destination: PathBuf,
    },
    Probe,
}

pub fn run_cli_command(command: Command) -> Result<(), anyhow::Error> {
    match command {
        Command::Export {
            elevation_low,
            elevation_high,
            destination,
        } => export(elevation_low, elevation_high, destination),
        Command::Probe => probe(),
    }
}

fn export(
    elevation_low: i32,
    elevation_high: i32,
    destination: PathBuf,
) -> Result<(), anyhow::Error> {
    let pb = ProgressBar::new(1);
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar:.cyan/blue}]")
            .unwrap()
            .progress_chars("#>-"),
    );
    let mut df = dfhack_remote::connect()?;
    let range = elevation_low..elevation_high;
    let (progress_tx, progress_rx) = std::sync::mpsc::channel();
    let (_cancel_tx, cancel_rx) = std::sync::mpsc::channel();
    let task = thread::spawn(move || {
        export::export_voxels(&mut df, range, destination, progress_tx, cancel_rx);
    });
    'outer: loop {
        for progress in progress_rx.try_iter() {
            match progress {
                export::Progress::Connecting => {}
                export::Progress::StartReading { total } => {
                    pb.println("[1/3] Reading the fortress...");
                    pb.set_length(total as u64);
                }
                export::Progress::Reading { curr, to: _ } => {
                    pb.set_position(curr as u64);
                }
                export::Progress::StartBuilding { total } => {
                    pb.println("[2/3] Building the model...");
                    pb.set_length(total as u64);
                }
                export::Progress::Building { curr, to: _ } => {
                    pb.set_position(curr as u64);
                }
                export::Progress::Writing => {
                    pb.println("[3/3] Saving the model...");
                }
                export::Progress::Done { path } => {
                    pb.println(format!("Sucessfully saved to {}", path.to_string_lossy()));
                    pb.finish_and_clear();
                    break 'outer;
                }
                export::Progress::Error(e) => {
                    pb.println(e.to_string());
                    pb.abandon();
                    break 'outer;
                }
            }
        }
    }
    task.join().unwrap();
    Ok(())
}

fn probe() -> Result<(), anyhow::Error> {
    let mut client = dfhack_remote::connect()?;
    let view_info = client.remote_fortress_reader().get_view_info()?;
    let x = view_info.cursor_pos_x();
    let y = view_info.cursor_pos_y();
    let z = view_info.cursor_pos_z();
    let tile_type_list = client.remote_fortress_reader().get_tiletype_list()?;
    let tile_types = &tile_type_list.tiletype_list;
    let material_list = client.remote_fortress_reader().get_material_list()?;
    let materials = &material_list.material_list;
    let (_, tiles) = tile_iterator::iter_tiles(
        &mut client,
        100,
        0..1000,
        0..1000,
        z..z + 1,
        tile_types,
        materials,
    )?;
    for tile in tiles {
        let tile = tile?;
        if (tile.coords.x, tile.coords.y, tile.coords.z) == (x, y, z) {
            dbg!(tile);
        }
    }
    Ok(())
}
