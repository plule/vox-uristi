use std::{
    fmt::Display,
    ops::{Add, Sub},
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

use super::{widgets::*, TermColor, THEME};
use crate::export::{Cancel, Progress};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{Gauge, Paragraph, StatefulWidget, Widget, Wrap},
};
use tui_textarea::TextArea;

/// E-gui like immediate UI, with drawing and input handling in the same functions.
pub struct Tuian<'a, 'b> {
    /// Drawing frame
    pub frame: &'a mut Frame<'b>,

    /// Building focus, incremented as the UI is built
    pub next_focus: usize,

    /// Event from the user
    pub event: Option<Event>,

    /// Instructions for the user
    pub instructions: Vec<String>,
}

/// Response from drawing a widget
#[derive(Default)]
pub struct Response {
    pub event: Option<Event>,
    pub modified: bool,
}

impl Response {
    fn empty() -> Self {
        Self {
            event: None,
            modified: false,
        }
    }

    pub fn with_event(mut self, event: Option<Event>) -> Self {
        self.event = event;
        self
    }

    pub fn with_modified(mut self) -> Self {
        self.modified = true;
        self
    }

    pub fn pressed_key_kode(&self) -> Option<KeyCode> {
        if let Some(Event::Key(KeyEvent {
            code: k,
            kind: KeyEventKind::Press,
            ..
        })) = self.event
        {
            Some(k)
        } else {
            None
        }
    }

    pub fn entered(&self) -> bool {
        self.pressed_key_kode() == Some(KeyCode::Enter)
    }

    pub fn pressed(&self, key: char) -> bool {
        self.pressed_key_kode() == Some(KeyCode::Char(key))
    }
}

pub enum Popup {
    Destination,
    Progress(Progress, Receiver<Progress>, Sender<Cancel>),
}

/// Status of the UI focus
pub enum Focus {
    Regular(usize),
    Popup { previous_focus: usize, popup: Popup },
}

impl Default for Focus {
    fn default() -> Self {
        Self::Regular(0)
    }
}

impl Focus {
    pub fn regular_focus(&self) -> Option<usize> {
        match self {
            Self::Regular(focus) => Some(*focus),
            Self::Popup { .. } => None,
        }
    }

    pub fn reset_to_regular(&mut self) {
        match self {
            Self::Regular(_) => {}
            Self::Popup { previous_focus, .. } => {
                *self = Self::Regular(*previous_focus);
            }
        }
    }

    pub fn update_from_event(&mut self, event: &Option<Event>) {
        if let Some(Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        })) = event
        {
            match self {
                Focus::Regular(focus) => match code {
                    KeyCode::Up => {
                        if *focus > 0 {
                            *focus -= 1;
                        }
                    }
                    KeyCode::Down => {
                        *focus += 1;
                    }
                    _ => {}
                },
                Focus::Popup { .. } => {
                    if *code == KeyCode::Esc {
                        self.reset_to_regular();
                    }
                }
            }
        }
    }
}

impl<'a, 'b> Tuian<'a, 'b> {
    pub fn clamp_focus(&self, focus: &mut Focus) {
        match focus {
            Focus::Regular(f) => {
                *focus = Focus::Regular((*f).min(self.next_focus - 1));
            }
            Focus::Popup { .. } => {}
        }
    }
    pub fn render_widget<W>(&mut self, widget: W, area: Rect)
    where
        W: Widget,
    {
        self.frame.render_widget(widget, area);
    }

    pub fn render_stateful_widget<W>(&mut self, widget: W, area: Rect, state: &mut W::State)
    where
        W: StatefulWidget,
    {
        self.frame.render_stateful_widget(widget, area, state);
    }

    /// Check if the widget is focused, and increment the internal focus count
    pub fn take_next_focus(&mut self, focus: &Focus) -> bool {
        let has_focus = focus.regular_focus().is_some_and(|f| f == self.next_focus);
        self.next_focus += 1;
        has_focus
    }

    pub fn text(&mut self, text: impl Into<String>, area: Rect, color: Color) {
        let text = Paragraph::new(text.into())
            .style(Style::default().fg(color))
            .wrap(Wrap { trim: true });
        self.render_widget(text, area);
    }

    pub fn error(&mut self, text: impl Into<String>, area: Rect) {
        self.text(text, area, THEME.red().term_color())
    }

    pub fn body(&mut self, text: impl Into<String>, area: Rect) {
        self.text(text, area, THEME.text().term_color())
    }

    pub fn text_subtle(&mut self, text: impl Into<String>, area: Rect) {
        self.text(text, area, THEME.overlay1().term_color())
    }

    /// Draw a button and return true if it was pressed.
    pub fn button<I: IntoIterator<Item = impl Into<String>>>(
        &mut self,
        focus: &Focus,
        text: impl Into<String>,
        instructions: I,
        area: Rect,
        style: Style,
    ) -> Response {
        let has_focus = self.take_next_focus(focus);
        let button = Button::new(has_focus, text.into(), style);
        self.render_widget(button, area);

        if has_focus {
            self.instructions
                .extend(instructions.into_iter().map(|s| s.into()));
            Response::empty().with_event(self.event.take())
        } else {
            Response::empty()
        }
    }

    /// Draw a number input and return true if it was modified
    pub fn number_input<
        I: IntoIterator<Item = impl Into<String>>,
        T: Add<i32, Output = T> + Sub<i32, Output = T> + Display + Copy,
    >(
        &mut self,
        focus: &Focus,
        text: impl Into<String>,
        instructions: I,
        value: &mut T,
        value_color: Color,
        area: Rect,
    ) -> Response {
        let has_focus = self.take_next_focus(focus);
        let widget = PromptWidget::new(
            has_focus,
            text.into(),
            Paragraph::new(value.to_string()).style(
                Style::default()
                    .fg(value_color)
                    .add_modifier(Modifier::BOLD),
            ),
        );
        self.render_widget(widget, area);
        if has_focus {
            self.instructions.push("↔: Change value".to_string());
            self.instructions
                .extend(instructions.into_iter().map(|s| s.into()));
            if let Some(event) = self.event.take() {
                match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Right,
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        *value = *value + 1;
                        return Response::default().with_modified();
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Left,
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        *value = *value - 1;
                        return Response::default().with_modified();
                    }
                    _ => {
                        return Response::default().with_event(Some(event));
                    }
                }
            }
        }
        Response::empty()
    }

    pub fn path_pick_popup<F>(
        &mut self,
        textarea: &mut TextArea<'static>,
        validation: F,
        area: Rect,
    ) -> Option<PathBuf>
    where
        F: FnOnce(&str) -> Validation<PathBuf>,
    {
        let validated = validation(&textarea.lines()[0]);

        let (color, status) = match &validated {
            Validation::Valid(_) => (THEME.green().term_color(), "OK".to_string()),
            Validation::Warning(_, warning) => (THEME.yellow().term_color(), warning.to_string()),
            Validation::Error(error) => (THEME.red().term_color(), error.to_string()),
        };
        let path_pick = PathPick::new(validated.clone());
        let widget = PopupWidget::<PathPick> {
            widget: path_pick,
            title: "Save as...".to_string(),
            bottom_text_color: color,
            bottom_text: status,
        };
        self.render_stateful_widget(widget, area, textarea);

        self.instructions.push("Enter: OK".to_string());
        if let Some(event) = self.event.take() {
            match (&event, validated) {
                (
                    Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        kind: KeyEventKind::Press,
                        ..
                    }),
                    Validation::Valid(path),
                ) => {
                    return Some(path.clone());
                }
                _ => {
                    textarea.input(event);
                }
            }
        }
        None
    }

    pub fn progress_bar_popup(&mut self, progress: &Progress, area: Rect) {
        let widget: PopupWidget<Gauge> = PopupWidget::<Gauge> {
            widget: ratatui::widgets::Gauge::default()
                .gauge_style(Style::default())
                .ratio(progress.ratio()),
            title: "Exporting...".to_string(),
            bottom_text_color: THEME.overlay1().term_color(),
            bottom_text: progress.text(),
        };
        self.render_widget(widget, area);
    }
}

#[derive(Clone)]
pub enum Validation<T: Clone> {
    Valid(T),
    Warning(T, &'static str),
    Error(&'static str),
}
