use crate::{
    calendar::{Month, TimeOfTheYear},
    export::{self, run_export_thread, Elevation, ExportParams},
    rfr::DFHackExt,
};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use strum::IntoEnumIterator;

#[cfg(feature = "dev")]
pub mod dev;

pub fn export(
    low: Option<Elevation>,
    high: Option<Elevation>,
    path: PathBuf,
    month: Option<Month>,
) -> Result<()> {
    let pb = ProgressBar::new(1);
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar:.cyan/blue}]")
            .unwrap()
            .progress_chars("#>-"),
    );
    let mut df = dfhack_remote::connect()?;
    let time = match month {
        Some(month) => TimeOfTheYear::Month(month),
        None => TimeOfTheYear::Current,
    };

    let (elevation_low, elevation_high) = match (low, high) {
        (Some(low), Some(high)) => (low, high),
        (Some(elevation), None) | (None, Some(elevation)) => (elevation, elevation),
        (None, None) => {
            let elevation = Elevation(df.elevation()?);
            (elevation, elevation)
        }
    };
    let (progress_rx, _cancel_tx, handle) = run_export_thread(
        ExportParams {
            elevation_low,
            elevation_high,
            time,
            path,
        },
        Some(df),
    );

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
    handle.join().unwrap();
    Ok(())
}

pub fn export_year(
    elevation_low: Option<Elevation>,
    elevation_high: Option<Elevation>,
    destination: PathBuf,
) -> Result<()> {
    for (index, month) in Month::iter().enumerate() {
        let mut destination = destination.clone();
        destination.push(format!("{:02}-{}.vox", index + 1, month));
        export(elevation_low, elevation_high, destination, Some(month))?;
    }
    Ok(())
}

#[cfg(feature = "self-update")]
pub fn check_update() -> Result<()> {
    use crate::update;
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
