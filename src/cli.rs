use crate::{calendar::Month, export, rfr, update, DFCoords};
use anyhow::Result;
use clap::{Parser, Subcommand};
use dfhack_remote::{BasicMaterialInfoMask, BlockRequest, ListMaterialsIn};
use indicatif::{ProgressBar, ProgressStyle};
use protobuf::{Message, MessageDyn, MessageField};
use std::{path::PathBuf, thread};
use strum::IntoEnumIterator;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Export the map in the .vox folder
    Export {
        /// Lower point to export
        #[arg(long)]
        low: Option<i32>,
        /// Higher point to export
        #[arg(long)]
        high: Option<i32>,
        /// Season for export
        #[arg(short, long)]
        month: Option<Month>,
        /// Destination file
        destination: PathBuf,
    },
    ExportYear {
        /// Lower point to export
        #[arg(short, long)]
        low: Option<i32>,
        /// Higher point to export
        #[arg(short, long)]
        high: Option<i32>,
        /// Destination folder
        destination: PathBuf,
    },
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
    /// Check for new versions
    CheckUpdate,
}

pub fn run_cli_command(command: Command) -> Result<()> {
    match command {
        Command::Export {
            low: elevation_low,
            high: elevation_high,
            destination,
            month,
        } => export(elevation_low, elevation_high, destination, month),
        Command::ExportYear {
            low: elevation_low,
            high: elevation_high,
            destination,
        } => export_year(elevation_low, elevation_high, destination),
        Command::DumpLists { destination } => dump_lists(destination),
        Command::Probe { destination } => probe(destination),
        Command::CheckUpdate => check_update(),
        Command::RegenTestData => regen_test_data(),
    }
}

fn export(
    low: Option<i32>,
    high: Option<i32>,
    destination: PathBuf,
    month: Option<Month>,
) -> Result<()> {
    let pb = ProgressBar::new(1);
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar:.cyan/blue}]")
            .unwrap()
            .progress_chars("#>-"),
    );
    let mut df = dfhack_remote::connect()?;
    let year_tick = match month {
        Some(month) => month.year_tick(),
        None => df.remote_fortress_reader().get_world_map()?.cur_year_tick(),
    };
    let (low, high) = match (low, high) {
        (Some(low), Some(high)) => (low, high),
        (Some(elevation), None) | (None, Some(elevation)) => (elevation, elevation),
        (None, None) => {
            let elevation = df.remote_fortress_reader().get_view_info()?.view_pos_z();
            (elevation - 2, elevation)
        }
    };
    let range = low..high + 1;
    let (progress_tx, progress_rx) = std::sync::mpsc::channel();
    let (_cancel_tx, cancel_rx) = std::sync::mpsc::channel();
    let task = thread::spawn(move || {
        export::export_voxels(
            &mut df,
            range,
            year_tick,
            destination,
            progress_tx,
            cancel_rx,
        );
    });
    'outer: loop {
        for progress in progress_rx.try_iter() {
            match progress {
                export::Progress::Undetermined { message } => {
                    pb.println(message);
                }
                export::Progress::Start { message, total } => {
                    pb.println(message);
                    pb.set_length(total as u64);
                }
                export::Progress::Progress {
                    message: _,
                    curr,
                    total: _,
                } => {
                    pb.set_position(curr as u64);
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

fn export_year(
    elevation_low: Option<i32>,
    elevation_high: Option<i32>,
    destination: PathBuf,
) -> Result<()> {
    for (index, month) in Month::iter().enumerate() {
        let mut destination = destination.clone();
        destination.push(format!("{:02}-{}.vox", index + 1, month));
        export(elevation_low, elevation_high, destination, Some(month))?;
    }
    Ok(())
}

fn probe(destination: PathBuf) -> Result<(), anyhow::Error> {
    let mut client = dfhack_remote::connect()?;
    let view_info = client.remote_fortress_reader().get_view_info()?;
    let x = view_info.cursor_pos_x();
    let y = view_info.cursor_pos_y();
    let z = view_info.cursor_pos_z();
    let tile_type_list = client.remote_fortress_reader().get_tiletype_list()?;
    let probe = DFCoords::new(x, y, z);
    for block_list in rfr::BlockListIterator::try_new(&mut client, 100, 0..1000, 0..1000, z..z + 1)?
    {
        for block in block_list?.map_blocks {
            for tile in rfr::TileIterator::new(&block, &tile_type_list) {
                if tile.coords() == probe {
                    println!("{}", tile);
                }
            }

            for (i, building) in block.buildings.into_iter().enumerate() {
                let bx = building.pos_x_min()..=building.pos_x_max();
                let by = building.pos_y_min()..=building.pos_y_max();
                let bz = building.pos_z_min()..=building.pos_z_max();
                if building.room.is_none() && bx.contains(&x) && by.contains(&y) && bz.contains(&z)
                {
                    dump(
                        &building,
                        &destination,
                        format!("building_{i}.json").as_str(),
                    )?;
                }
            }
            for (i, flow) in block.flows.iter().enumerate() {
                if DFCoords::from(flow.pos.get_or_default()) == probe {
                    dump(flow, &destination, format!("flow_{i}.json").as_str())?;
                }
            }
        }
    }

    Ok(())
}

fn regen_test_data() -> Result<(), anyhow::Error> {
    let destination = PathBuf::from("testdata");
    let mut client = dfhack_remote::connect()?;
    client.remote_fortress_reader().reset_map_hashes()?;
    let view_info = client.remote_fortress_reader().get_view_info()?;
    let z = view_info.cursor_pos_z();
    for (index, block_list) in
        rfr::BlockListIterator::try_new(&mut client, 100, 0..1000, 0..1000, z..z + 1)?.enumerate()
    {
        let data = block_list?.write_to_bytes()?;
        let mut dest = destination.clone();
        dest.push(format!("block_{index}.dat"));
        println!("{}", &dest.display());
        std::fs::write(dest, data)?;
    }

    let building_defs = client.remote_fortress_reader().get_building_def_list()?;
    let data = building_defs.write_to_bytes()?;
    let mut dest = destination.clone();
    dest.push(format!("building_defs.dat"));
    println!("{}", &dest.display());
    std::fs::write(dest, data)?;

    Ok(())
}

fn dump_lists(destination: PathBuf) -> Result<()> {
    let mut client = dfhack_remote::connect()?;

    let req = ListMaterialsIn {
        mask: MessageField::some(BasicMaterialInfoMask {
            flags: Some(true),
            reaction: Some(true),
            ..Default::default()
        }),
        inorganic: Some(true),
        builtin: Some(true),
        ..Default::default()
    };

    let basic_materials = client.core().list_materials(req)?;
    dump(&basic_materials, &destination, "basic_materials.json")?;

    let materials = client.remote_fortress_reader().get_material_list()?;
    dump(&materials, &destination, "materials.json")?;

    let plants = client.remote_fortress_reader().get_plant_raws()?;
    dump(&plants, &destination, "plant_raws.json")?;

    let ttypes = client.remote_fortress_reader().get_tiletype_list()?;
    dump(&ttypes, &destination, "tiletypes.json")?;

    let building_defs = client.remote_fortress_reader().get_building_def_list()?;
    dump(&building_defs, &destination, "building_defs.json")?;

    let growth_list = client.remote_fortress_reader().get_growth_list()?;
    dump(&growth_list, &destination, "growths.json")?;

    let item_list = client.remote_fortress_reader().get_item_list()?;
    dump(&item_list, &destination, "items.json")?;

    let language = client.remote_fortress_reader().get_language()?;
    dump(&language, &destination, "language.json")?;

    let view_info = client.remote_fortress_reader().get_view_info()?;
    client.remote_fortress_reader().reset_map_hashes()?;
    let z = view_info.cursor_pos_z();
    let req = BlockRequest {
        blocks_needed: Some(1),
        min_x: Some(0),
        max_x: Some(1000),
        min_y: Some(0),
        max_y: Some(1000),
        min_z: Some(z),
        max_z: Some(z + 1),
        ..Default::default()
    };
    let blocks = client.remote_fortress_reader().get_block_list(req)?;
    dump(&blocks, &destination, "blocks.json")?;

    let enums = client.core().list_enums()?;
    dump(&enums, &destination, "enums.json")?;

    Ok(())
}

fn dump(message: &dyn MessageDyn, folder: &PathBuf, filename: &str) -> Result<()> {
    let json = protobuf_json_mapping::print_to_string(message)?;
    let mut dest = folder.clone();
    dest.push(filename);
    println!("{}", &dest.display());
    std::fs::write(dest, json)?;
    Ok(())
}

fn check_update() -> Result<()> {
    match update::check_update()? {
        update::UpdateStatus::UpToDate => {
            println!("Up to date");
        }
        update::UpdateStatus::NewVersion {
            name,
            release_url,
            asset_url,
        } => {
            println!("Vox Uristi {name} is available");
            println!("URL: {release_url}");
            if let Some(asset_url) = asset_url {
                println!("Download: {asset_url}");
            }
        }
    };

    Ok(())
}
