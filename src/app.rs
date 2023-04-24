use crate::export::{export_voxels, Cancel, Progress};
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

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct App {
    #[serde(skip)]
    error: Option<String>,
    #[serde(skip)]
    progress: Option<(Progress, Receiver<Progress>, Sender<Cancel>)>,
    #[serde(skip)]
    exported_path: Option<String>,
    low_elevation: i32,
    high_elevation: i32,
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
                    if (ui.add(elevation_picker("⏶".to_string(), &mut self.high_elevation)))
                        .changed()
                    {
                        self.low_elevation = self.low_elevation.min(self.high_elevation - 1);
                    }
                    if ui
                        .add(elevation_picker("⏷".to_string(), &mut self.low_elevation))
                        .changed()
                    {
                        self.high_elevation = self.high_elevation.max(self.low_elevation + 1);
                    }
                    ui.separator();
                    let button = Button::new(RichText::new("💾 Export").heading());
                    if ui
                        .add_sized(Vec2::new(ui.available_width(), 40.0), button)
                        .clicked()
                    {
                        self.error = None;
                        match dfhack_remote::connect() {
                            Ok(mut df) => {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_title("Model destination")
                                    .set_file_name(&default_filename(&mut df))
                                    .save_file()
                                {
                                    let (progress_tx, progress_rx) = std::sync::mpsc::channel();
                                    let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();
                                    let range = self.low_elevation..self.high_elevation;
                                    self.progress =
                                        Some((Progress::Connecting, progress_rx, cancel_tx));
                                    thread::spawn(move || {
                                        export_voxels(&mut df, range, path, progress_tx, cancel_rx);
                                    });
                                }
                            }
                            Err(err) => {
                                self.error = Some(err.to_string());
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
}

impl Default for App {
    fn default() -> Self {
        Self {
            error: None,
            progress: None,
            exported_path: None,
            low_elevation: 100,
            high_elevation: 110,
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.central_panel(ui, ctx);
        });
    }
}

fn default_filename(df: &mut dfhack_remote::Stubs<dfhack_remote::Channel>) -> String {
    maybe_default_filename(df).unwrap_or_else(|| "model.vox".to_string())
}

fn maybe_default_filename(df: &mut dfhack_remote::Stubs<dfhack_remote::Channel>) -> Option<String> {
    let world_map = df.remote_fortress_reader().get_world_map().ok()?;
    Some(format!(
        "{}_{}.vox",
        world_map.name_english(),
        world_map.cur_year()
    ))
}

fn try_get_current_elevation() -> Result<i32> {
    let mut df = dfhack_remote::connect()?;
    let view = df.remote_fortress_reader().get_view_info()?;
    Ok(view.view_pos_z())
}

fn elevation_picker(text: String, elevation: &mut i32) -> impl egui::Widget + '_ {
    move |ui: &mut Ui| {
        ui.horizontal(|ui| {
            ui.label(text);
            let mut resp = ui.add(DragValue::new(elevation).clamp_range(0..=300));
            if ui
                .button("⛶ Current")
                .on_hover_text("Set the elevation from the current view.")
                .clicked()
            {
                if let Ok(current_elevation) = try_get_current_elevation() {
                    resp.mark_changed();
                    *elevation = current_elevation;
                }
            }
            resp
        })
        .inner
    }
}
