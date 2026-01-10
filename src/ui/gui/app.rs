#[cfg(feature = "self-update")]
use crate::{
    calendar::{Month, TimeOfTheYear},
    export::{run_export_thread, Cancel, Elevation, Progress},
    ui::State,
    FromDwarfFortress,
};
use anyhow::{anyhow, Context, Result};
use eframe::{
    egui::{self, Button, DragValue, ProgressBar, Response, RichText, Ui},
    epaint::Vec2,
};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::ui;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct App {
    state: crate::ui::State,
    #[serde(skip)]
    df: Result<dfhack_remote::Client>,
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
        match &mut self.state.progress {
            Some((progress, rx, tx)) => {
                ctx.request_repaint();
                if ui.button("Cancel").clicked() {
                    canceled = true;
                    if let Err(err) = tx.send(Cancel) {
                        self.state.error = Some(format!("Failed to cancel: {err}"));
                    }
                }
                if let Some(new_progress) = rx.try_iter().last() {
                    *progress = new_progress;
                }
                ui.label("Do not unpause the game during the export.");
                match progress {
                    Progress::Undetermined { message } => {
                        ui.label(*message);
                        ui.spinner();
                    }
                    Progress::Update {
                        message,
                        curr,
                        total,
                    } => {
                        ui.add(ProgressBar::new(*curr as f32 / *total as f32).text(*message));
                    }
                    Progress::Start {
                        message: _,
                        total: _,
                    } => {}
                    Progress::Done { path } => {
                        self.state.exported_path = Some(path.to_path_buf());
                        self.state.progress = None;
                    }
                    Progress::Error(err) => {
                        self.state.error = Some(err.to_string());
                        self.state.progress = None;
                    }
                }
            }
            None => {
                ui.group(|ui| {
                    ui.add(df_client_group(&mut self.df, |ui, df| {
                        ui.label("Pick the elevation range to export");
                        ui.label("It works best by covering the surface level.");
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width());
                        });
                        if elevation_picker(ui, "⏶", &mut self.state.high_elevation, df)?.changed()
                        {
                            self.state.low_elevation.0 =
                                self.state.low_elevation.0.min(self.state.high_elevation.0);
                        };
                        if elevation_picker(ui, "⏷", &mut self.state.low_elevation, df)?.changed()
                        {
                            self.state.high_elevation.0 =
                                self.state.high_elevation.0.max(self.state.low_elevation.0);
                        }

                        time_picker(ui, &mut self.state.time, df)?;
                        ui.separator();
                        let button = Button::new(RichText::new("💾 Export").heading());
                        if ui
                            .add_sized(Vec2::new(ui.available_width(), 40.0), button)
                            .clicked()
                        {
                            self.state.error = None;
                            let world_map = df.remote_fortress_reader().get_world_map()?;
                            let file_name = format!(
                                "{}_{}.vox",
                                world_map.name_english(),
                                world_map.cur_year()
                            );

                            if let Some(path) = rfd::FileDialog::new()
                                .set_title("Model destination")
                                .set_file_name(file_name)
                                .add_filter("MagicaVoxel", &["vox"])
                                .save_file()
                            {
                                let (progress_rx, cancel_tx, _) =
                                    run_export_thread(self.state.export_params(path), None);
                                self.state.progress = Some((
                                    Progress::undetermined("Connecting..."),
                                    progress_rx,
                                    cancel_tx,
                                ));
                            }
                        }
                        Ok(())
                    }));
                });
            }
        }
        if canceled {
            self.state.progress = None;
        }

        if let Some(path) = &self.state.exported_path {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("🗁 Show in explorer").clicked() {
                        if let Err(err) = opener::reveal(path) {
                            self.state.error = Some(err.to_string());
                        }
                    }
                    ui.label(format!(
                        "'{}' exported",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                    ui.add_space(ui.available_width());
                });
            });
        }

        if let Some(err) = &self.state.error {
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
        #[cfg(feature = "self-update")]
        ui.horizontal(|ui| match &self.state.update_status {
            ui::CheckUpdateStatus::NotDone => {
                let (sender, receiver) = std::sync::mpsc::channel();
                self.state.update_status = ui::CheckUpdateStatus::Doing(receiver);
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    sender.send(crate::update::check_update()).unwrap();
                    ctx.request_repaint();
                });
            }
            ui::CheckUpdateStatus::Doing(_) => {
                ui.spinner();
            }
            ui::CheckUpdateStatus::Done(crate::update::UpdateStatus::UpToDate) => {
                ui.label("✔ Up to date");
            }
            ui::CheckUpdateStatus::Done(crate::update::UpdateStatus::NewVersion {
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
            ui::CheckUpdateStatus::Failed(err) => {
                ui.label(format!("Could not check update: {err}"));
            }
        });
    }
}

impl Default for App {
    fn default() -> Self {
        let df = match dfhack_remote::connect() {
            Ok(df) => Ok(df),
            Err(err) => Err(anyhow!(err)),
        };
        Self {
            state: State::default(),
            df,
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "self-update")]
        if let ui::CheckUpdateStatus::Doing(receiver) = &self.state.update_status {
            if let Some(update_status) = receiver.try_iter().last() {
                match update_status {
                    Ok(update_status) => {
                        self.state.update_status = ui::CheckUpdateStatus::Done(update_status);
                    }
                    Err(err) => {
                        self.state.update_status = ui::CheckUpdateStatus::Failed(err.to_string());
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

fn elevation_picker(
    ui: &mut Ui,
    text: &str,
    elevation: &mut Elevation,
    df: &mut dfhack_remote::Client,
) -> Result<Response> {
    ui.horizontal(|ui| {
        ui.label(text);
        let button = ui
            .button("☉")
            .on_hover_text("Set the elevation from the current view.");
        if button.clicked() {
            elevation.read_from_df(df)?;
        }
        let mut resp = ui
            .add(DragValue::new(&mut elevation.0).range(-200..=200))
            .on_hover_text("Defines the elevation range that will be exported.");
        if button.clicked() {
            resp.mark_changed();
        }
        Ok(resp)
    })
    .inner
}

fn time_picker(
    ui: &mut Ui,
    time: &mut TimeOfTheYear,
    df: &mut dfhack_remote::Client,
) -> Result<()> {
    ui.horizontal(|ui| {
        ui.label("📆");
        if ui
            .button("☉")
            .on_hover_text("Set the time of the year to the current time.")
            .clicked()
        {
            time.read_from_df(df)?;
        }
        egui::ComboBox::from_label("")
            .selected_text(format!("{}", time))
            .show_ui(ui, |ui| {
                for month in Month::iter() {
                    let text = egui::RichText::new(format!("{}", month)).color(month.gui_color());
                    ui.selectable_value(time, TimeOfTheYear::Month(month), text);
                }
            }).response.on_hover_text("Define the time of the year of the export. This affects the vegetation appearance.");

        Ok(())
    })
    .inner
}

fn df_client_group<'a, R>(
    df: &'a mut Result<dfhack_remote::Client>,
    add_contents: impl FnOnce(&mut Ui, &mut dfhack_remote::Client) -> Result<R> + 'a,
) -> impl egui::Widget + 'a {
    move |ui: &mut Ui| {
        let mut new_df = None;
        let response = match df {
            Ok(df) => {
                ui.add_enabled_ui(true, |ui| {
                    if let Err(err) = add_contents(ui, df) {
                        new_df = Some(Err(err));
                    }
                })
                .response
            }
            Err(err) => ui.vertical(|ui| {
                ui.label("Failed to communicate with Dwarf Fortress. Is it running with DFHack installed?");
                ui.label(err.to_string());
                if ui.button("Reconnect").clicked() {
                    new_df = Some(dfhack_remote::connect().context("Connecting to DFHack"));
                }
            }).response,
        };

        if let Some(new_df) = new_df {
            *df = new_df;
        }

        response
    }
}
