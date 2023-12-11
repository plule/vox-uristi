use crate::{export::Progress, VERSION};

use anyhow::Result;
use catppuccin::Flavour;
use crossterm::{
    event,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::{CrosstermBackend, Terminal},
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, Borders, Padding},
    Frame,
};
use std::{ffi::OsStr, io::stdout, path::PathBuf};
use tui_textarea::TextArea;

use self::widgets::ZoneBlock;

use super::{FromDwarfFortress, State};
mod tuian;
mod widgets;
use tuian::*;

pub const THEME: Flavour = Flavour::Latte;

pub trait TermColor {
    fn term_color(&self) -> Color;
}

impl TermColor for catppuccin::Colour {
    fn term_color(&self) -> Color {
        Color::Rgb(self.0, self.1, self.2)
    }
}

enum FutureDF {
    Connecting(Option<std::thread::JoinHandle<dfhack_remote::Result<dfhack_remote::Client>>>),
    Connected(dfhack_remote::Result<dfhack_remote::Client>),
}

impl Default for FutureDF {
    fn default() -> Self {
        Self::Connecting(Some(std::thread::spawn(dfhack_remote::connect)))
    }
}

impl FutureDF {
    fn update(&mut self) {
        match self {
            Self::Connecting(handle) => {
                if let Some(handle) = handle.take() {
                    if handle.is_finished() {
                        *self = Self::Connected(handle.join().unwrap())
                    } else {
                        *self = Self::Connecting(Some(handle))
                    }
                } else {
                    *self = Self::Connecting(Default::default())
                }
            }
            Self::Connected(_) => {}
        }
    }
}

struct App {
    df: FutureDF,

    state: State,

    destination_area: TextArea<'static>,

    focus: Focus,
}

fn validate_path(input: &str) -> Validation<PathBuf> {
    if input.is_empty() {
        return Validation::Error("Empty path");
    }

    let path = PathBuf::from(input);
    if path.exists() {
        return Validation::Error("File exists");
    }

    if path.extension() != Some(OsStr::new("vox")) {
        return Validation::Warning(path, "File extension is not `.vox`. Are you sure?");
    }

    Validation::Valid(path)
}

impl Default for App {
    fn default() -> Self {
        Self {
            df: Default::default(),
            state: Default::default(),
            destination_area: Default::default(),
            focus: Default::default(),
        }
    }
}

impl App {
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        let mut exit = false;
        while !exit {
            terminal.draw(|f| {
                exit = self.ui(f);
            })?;
        }

        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) -> bool {
        let window_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Max(80), Constraint::Min(0)])
            .split(f.size());
        let window_area = window_layout[0];
        let mut event = None;
        if event::poll(std::time::Duration::from_millis(16)).unwrap() {
            event = event::read().ok();
            self.focus.update_from_event(&event);
        }
        let mut ui = Tuian {
            frame: f,
            next_focus: 0,
            event,
            instructions: vec!["↕: Focus".to_string()],
        };

        // Render the background first
        let background = Block::default()
            .bg(THEME.base().term_color())
            .padding(Padding::new(2, 2, 2, 2));
        let content_area = background.inner(window_area);
        ui.render_widget(background, window_area);

        self.df.update();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                // General UI
                Constraint::Min(10),
                // Reconnect button
                Constraint::Length(3),
                // Exit button
                Constraint::Length(3),
            ])
            .split(content_area);

        match &mut self.df {
            FutureDF::Connected(Ok(df)) => {
                Self::connected_ui(&mut self.state, &mut self.focus, chunks[0], &mut ui, df);
            }
            FutureDF::Connected(Err(err)) => {
                let err = err.to_string();
                Self::disconnected_ui(chunks[0], &mut ui, &err);
            }
            FutureDF::Connecting(_) => {
                ui.body("Connecting to Dwarf Fortress...", chunks[0]);
            }
        }

        if ui
            .button(
                &self.focus,
                "Reconnect",
                vec!["Enter: Reconnect"],
                chunks[1],
                Style::default().fg(THEME.lavender().term_color()),
            )
            .entered()
        {
            self.df = FutureDF::default();
        }

        if ui
            .button(
                &self.focus,
                "Exit",
                vec!["Enter: Exit"],
                chunks[2],
                Style::default().fg(THEME.red().term_color()),
            )
            .entered()
        {
            return true;
        }

        if let Focus::Popup { popup, .. } = &mut self.focus {
            let area = popup_rect(content_area);
            match popup {
                Popup::Destination => {
                    if let Some(destination) =
                        ui.path_pick_popup(&mut self.destination_area, validate_path, area)
                    {
                        let (progress_tx, progress_rx) = std::sync::mpsc::channel();
                        let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();
                        *popup = Popup::Progress(
                            Progress::undetermined("Connecting..."),
                            progress_rx,
                            cancel_tx,
                        );
                        let mut df = dfhack_remote::connect().unwrap();
                        let range = self.state.low_elevation.0..(self.state.high_elevation.0 + 1);
                        let ticks = self.state.time.ticks(&mut df);
                        let destination = destination.clone();
                        std::thread::spawn(move || {
                            crate::export::export_voxels(
                                &mut df,
                                range,
                                ticks,
                                destination,
                                progress_tx,
                                cancel_rx,
                            );
                        });
                    }
                }
                Popup::Progress(progress, progress_rx, _cancel_tx) => {
                    if let Some(new_progress) = progress_rx.try_iter().last() {
                        *progress = new_progress;
                    }
                    ui.progress_bar_popup(progress, area);
                }
            }
        }
        // Draw the window borders last
        // must not contain background
        let window = ZoneBlock::default()
            .title(format!("Vox Uristi v{VERSION}"))
            .title_color(THEME.peach().term_color())
            .bottom_text(ui.instructions.join(" | "))
            .borders(Borders::ALL)
            .border_type(BorderType::Double);

        ui.render_widget(window, window_area);

        ui.clamp_focus(&mut self.focus);
        false
    }

    fn disconnected_ui(content_area: Rect, ui: &mut Tuian<'_, '_>, err: &str) {
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                // Status
                Constraint::Length(3),
                // Detailed error
                Constraint::Length(3),
            ])
            .split(content_area);
        ui.error(
            "Failed to communicate with Dwarf Fortress. Is it running with DFHack installed?",
            content_chunks[0],
        );
        ui.error(err, content_chunks[1]);
    }

    fn connected_ui(
        state: &mut State,
        focus: &mut Focus,
        area: Rect,
        ui: &mut Tuian<'_, '_>,
        df: &mut dfhack_remote::Stubs<dfhack_remote::Channel>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                // Instructions
                Constraint::Length(3),
                // Top elevation
                Constraint::Length(1),
                // Low elevation
                Constraint::Length(1),
                // Time of the year
                Constraint::Length(1),
                // Export button
                Constraint::Length(3),
                // Status
                Constraint::Length(1),
            ])
            .split(area);
        ui.text_subtle(
            "Pick the elevation range to export. It works best by covering the surface layer.",
            chunks[0],
        );
        let mut res = ui.number_input(
            focus,
            "▲ High Elevation",
            vec!["r: Read frow Dwarf Fortress"],
            &mut state.high_elevation.0,
            THEME.text().term_color(),
            chunks[1],
        );
        if res.pressed('r') {
            state.high_elevation.do_read_from_df(df).unwrap();
            res.modified = true;
        }
        if res.modified {
            state.low_elevation.0 = state.low_elevation.0.min(state.high_elevation.0);
        }
        let mut res = ui.number_input(
            &focus,
            "▼ Low Elevation",
            vec!["r: Read frow Dwarf Fortress"],
            &mut state.low_elevation.0,
            THEME.text().term_color(),
            chunks[2],
        );
        if res.pressed('r') {
            state.low_elevation.do_read_from_df(df).unwrap();
            res.modified = true;
        }
        if res.modified {
            state.high_elevation.0 = state.high_elevation.0.max(state.low_elevation.0);
        }
        let month_color = state.time.tui_color();
        if ui
            .number_input(
                &focus,
                "☀ Time of the year",
                vec!["r: Read from Dwarf Fortress"],
                &mut state.time,
                month_color,
                chunks[3],
            )
            .pressed('r')
        {
            state.time.do_read_from_df(df).unwrap();
        }
        if ui
            .button(
                &focus,
                "Export",
                vec!["Enter: Export the fortress"],
                chunks[4],
                Style::default().fg(THEME.green().term_color()),
            )
            .entered()
        {
            *focus = Focus::Popup {
                previous_focus: ui.next_focus - 1,
                popup: Popup::Destination,
            };
        }
        Self::status(state, ui, chunks[5]);
    }

    fn status(state: &State, ui: &mut Tuian, area: Rect) {
        if let Some(error) = &state.error {
            ui.error(error, area);
        }
    }
}

fn popup_rect(area: Rect) -> Rect {
    let width = area.width - 4;
    let height = 3;
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    Rect::new(x, y, width, height)
}

pub fn run() -> Result<()> {
    let mut app = App::default();
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    app.run(&mut terminal)?;
    Ok(())
}
