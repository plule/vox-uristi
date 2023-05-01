use crate::{
    export::{export_voxels, Cancel, Progress},
    update::{check_update, UpdateStatus},
};
use anyhow::Result;
use eframe::{
    egui::{self, Button, DragValue, ProgressBar, RichText, Ui},
    epaint::Vec2,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::mpsc::{Receiver, Sender},
    thread,
};

enum CheckUpdateStatus {
    NotDone,
    Doing(Receiver<Result<UpdateStatus>>),
    Done(UpdateStatus),
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct App {
    low_elevation: i32,
    high_elevation: i32,

    #[serde(skip)]
    error: Option<String>,
    #[serde(skip)]
    progress: Option<(Progress, Receiver<Progress>, Sender<Cancel>)>,
    #[serde(skip)]
    exported_path: Option<String>,
    #[serde(skip)]
    update_status: CheckUpdateStatus,
    #[serde(skip)]
    df: Option<dfhack_remote::Client>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn central_panel(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.heading("☀Vox Uristi☀");

        let mut canceled = false;
        match &mut self.progress {
            Some((progress, rx, tx)) => {
                ctx.request_repaint();
                if ui.button("Cancel").clicked() {
                    canceled = true;
                    if let Err(err) = tx.send(Cancel) {
                        self.error = Some(format!("Failed to cancel: {err}"));
                    }
                }
                if let Some(new_progress) = rx.try_iter().last() {
                    *progress = new_progress;
                }
                ui.label("Do not unpause the game during the export.");
                match progress {
                    Progress::Connecting => {
                        ui.label("Starting");
                        ui.spinner();
                    }
                    Progress::Reading { curr, to } => {
                        ui.add(
                            ProgressBar::new(*curr as f32 / *to as f32)
                                .text("Reading the Fortress"),
                        );
                    }
                    Progress::Building { curr, to } => {
                        ui.add(
                            ProgressBar::new(*curr as f32 / *to as f32).text("Building the Model"),
                        );
                    }
                    Progress::Writing => {
                        ui.label("Saving the file");
                        ui.spinner();
                    }
                    Progress::Done { path } => {
                        self.exported_path = Some(path.to_string_lossy().to_string());
                        self.progress = None;
                    }
                    Progress::Error(err) => {
                        self.error = Some(err.to_string());
                        self.progress = None;
                    }
                    Progress::StartReading { total: _ } => {}
                    Progress::StartBuilding { total: _ } => {}
                }
            }
            None => {
                ui.group(|ui| {
                    ui.label("Pick the elevation range to export");
                    ui.label("It works best by covering the surface layer.");
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width());
                    });
                    if (ui.add(elevation_picker(
                        "⏶".to_string(),
                        &mut self.high_elevation,
                        &mut self.df,
                    )))
                    .changed()
                    {
                        self.low_elevation = self.low_elevation.min(self.high_elevation);
                    }
                    if ui
                        .add(elevation_picker(
                            "⏷".to_string(),
                            &mut self.low_elevation,
                            &mut self.df,
                        ))
                        .changed()
                    {
                        self.high_elevation = self.high_elevation.max(self.low_elevation);
                    }
                    ui.separator();
                    let button = Button::new(RichText::new("💾 Export").heading());
                    if ui
                        .add_sized(Vec2::new(ui.available_width(), 40.0), button)
                        .clicked()
                    {
                        self.error = None;
                        let df = match self.df.take() {
                            Some(df) => Ok(df),
                            None => dfhack_remote::connect(),
                        };
                        self.df = match df {
                            Ok(mut df) => {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_title("Model destination")
                                    .set_file_name(&default_filename(&mut df))
                                    .add_filter("MagicaVoxel", &["vox"])
                                    .save_file()
                                {
                                    let (progress_tx, progress_rx) = std::sync::mpsc::channel();
                                    let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();
                                    let range = self.low_elevation..self.high_elevation + 1;
                                    self.progress =
                                        Some((Progress::Connecting, progress_rx, cancel_tx));
                                    thread::spawn(move || {
                                        export_voxels(&mut df, range, path, progress_tx, cancel_rx);
                                    });
                                    None
                                } else {
                                    Some(df)
                                }
                            }
                            Err(err) => {
                                self.error = Some(err.to_string());
                                None
                            }
                        }
                    }
                });
            }
        }
        if canceled {
            self.progress = None;
        }

        if let Some(path) = &self.exported_path {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width());
                });
                ui.label("Fortress Exported");
                ui.horizontal(|ui| {
                    if ui.button("📋").on_hover_text("Click to copy").clicked() {
                        ui.output_mut(|o| o.copied_text = path.to_string());
                    }
                    ui.label(path);
                });
            });
        }

        if let Some(err) = &self.error {
            ui.label("Is Dwarf Fortress running with DFHack installed?");
            ui.label(err);
        }

        ui.collapsing("？ Information", |ui| {
            ui.hyperlink_to(" Source Code", "https://github.com/plule/vox-uristi");
            ui.hyperlink_to(
                " Dwarf Fortress",
                "https://store.steampowered.com/app/975370/Dwarf_Fortress",
            );
            ui.hyperlink_to(
                " DFHack",
                "https://store.steampowered.com/app/2346660/DFHack__Dwarf_Fortress_Modding_Engine",
            );
            ui.hyperlink_to("👁 MagicaVoxel", "https://ephtracy.github.io/");
        });
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| match &self.update_status {
            CheckUpdateStatus::NotDone => {
                if ui.button("🔃 Check for updates").clicked() {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    self.update_status = CheckUpdateStatus::Doing(receiver);
                    let ctx = ui.ctx().clone();
                    std::thread::spawn(move || {
                        sender.send(check_update()).unwrap();
                        ctx.request_repaint();
                    });
                }
            }
            CheckUpdateStatus::Doing(_) => {
                ui.spinner();
            }
            CheckUpdateStatus::Done(UpdateStatus::UpToDate) => {
                ui.label("✔ Up to date");
            }
            CheckUpdateStatus::Done(UpdateStatus::NewVersion {
                name,
                release_url,
                asset_url,
            }) => {
                ui.label(format!("⮉ {name} is available."));
                ui.horizontal(|ui| {
                    ui.hyperlink_to(" Open", release_url);
                    if let Some(asset_url) = asset_url {
                        ui.hyperlink_to("⬇ Download", asset_url);
                    }
                });
            }
        });
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            low_elevation: 100,
            high_elevation: 110,
            error: None,
            progress: None,
            exported_path: None,
            update_status: CheckUpdateStatus::NotDone,
            df: None,
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let CheckUpdateStatus::Doing(receiver) = &self.update_status {
            if let Some(update_status) = receiver.try_iter().last() {
                match update_status {
                    Ok(update_status) => {
                        self.update_status = CheckUpdateStatus::Done(update_status);
                    }
                    Err(err) => {
                        self.update_status = CheckUpdateStatus::NotDone;
                        self.error = Some(err.to_string());
                    }
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.central_panel(ui, ctx);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            self.status_bar(ui);
        });
    }
}

fn default_filename(df: &mut dfhack_remote::Client) -> String {
    maybe_default_filename(df).unwrap_or_else(|| "model.vox".to_string())
}

fn maybe_default_filename(df: &mut dfhack_remote::Client) -> Option<String> {
    let world_map = df.remote_fortress_reader().get_world_map().ok()?;
    Some(format!(
        "{}_{}.vox",
        world_map.name_english(),
        world_map.cur_year()
    ))
}

fn try_get_current_elevation(df: &mut Option<dfhack_remote::Client>) -> Result<i32> {
    let client = if let Some(current_client) = df {
        current_client
    } else {
        let new_client = dfhack_remote::connect()?;
        *df = Some(new_client);
        df.as_mut().unwrap()
    };
    match client.remote_fortress_reader().get_view_info() {
        Ok(view) => Ok(view.view_pos_z()),
        Err(err) => {
            *df = None;
            Err(anyhow::Error::from(err))
        }
    }
}

fn elevation_picker<'a>(
    text: String,
    elevation: &'a mut i32,
    df: &'a mut Option<dfhack_remote::Client>,
) -> impl egui::Widget + 'a {
    move |ui: &mut Ui| {
        ui.horizontal(|ui| {
            ui.label(text);
            let mut resp = ui.add(DragValue::new(elevation).clamp_range(0..=300));
            if ui
                .button("⛶ Current")
                .on_hover_text("Set the elevation from the current view.")
                .clicked()
            {
                if let Ok(current_elevation) = try_get_current_elevation(df) {
                    resp.mark_changed();
                    *elevation = current_elevation;
                }
            }
            resp
        })
        .inner
    }
}
