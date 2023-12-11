use std::path::PathBuf;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Styled},
    text::Line,
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, Clear, Paragraph, StatefulWidget, Widget,
    },
};
use tui_textarea::TextArea;

use super::{tuian::Validation, TermColor, THEME};

/// Display a prompt and a widget side by side.
pub struct PromptWidget<T: Widget + Styled> {
    pub focus: bool,
    pub prompt: String,
    pub widget: T,
}

/// Display a popup with a widget inside.
pub struct PopupWidget<T> {
    pub widget: T,
    pub title: String,
    pub bottom_text: String,
    pub bottom_text_color: Color,
}

/// Button pressable with enter
pub struct Button {
    pub focus: bool,
    pub text: String,
    pub style: Style,
}

pub struct PathPick {
    pub path: Validation<PathBuf>,
}

/// Block with a title and a bottom text.
pub struct ZoneBlock {
    pub title: String,
    pub title_style: Style,
    pub bottom_text: String,
    pub bottom_text_style: Style,
    pub borders: Borders,
    pub border_style: Style,
    pub border_type: BorderType,
    pub style: Style,
}

impl Default for ZoneBlock {
    fn default() -> Self {
        Self {
            title: Default::default(),
            title_style: Style::default().fg(THEME.text().term_color()),
            bottom_text: Default::default(),
            bottom_text_style: Style::default().fg(THEME.overlay1().term_color()),
            borders: Default::default(),
            border_style: Style::default().fg(THEME.overlay0().term_color()),
            border_type: Default::default(),
            style: Default::default(),
        }
    }
}

impl ZoneBlock {
    pub fn title(mut self, title: String) -> Self {
        self.title = title;
        self
    }

    pub fn title_color(mut self, title_color: Color) -> Self {
        self.title_style = self.title_style.fg(title_color);
        self
    }

    pub fn bottom_text(mut self, bottom_text: String) -> Self {
        self.bottom_text = bottom_text;
        self
    }

    pub fn bottom_text_color(mut self, bottom_text_color: Color) -> Self {
        self.bottom_text_style = self.bottom_text_style.fg(bottom_text_color);
        self
    }

    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    pub fn border_color(mut self, border_color: Color) -> Self {
        self.border_style = self.border_style.fg(border_color);
        self
    }

    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.border_type = border_type;
        self
    }

    pub fn bg(mut self, bg: Color) -> Self {
        self.style = self.style.bg(bg);
        self
    }

    pub fn block(self) -> Block<'static> {
        // Create the titles, with the decorations styled like the borders
        let title = Line::from(vec![
            "o ".set_style(self.border_style),
            self.title.set_style(self.title_style),
            " o".set_style(self.border_style),
        ]);

        let bottom_text = Line::from(vec![
            "o ".set_style(self.border_style),
            self.bottom_text.set_style(self.bottom_text_style),
            " o".set_style(self.border_style),
        ]);

        Block::default()
            .title(
                Title::from(title)
                    .position(Position::Top)
                    .alignment(Alignment::Center),
            )
            .title(
                Title::from(bottom_text)
                    .position(Position::Bottom)
                    .alignment(Alignment::Left),
            )
            .borders(self.borders)
            .border_type(self.border_type)
            .border_style(self.border_style)
            .style(self.style)
    }
}

impl Widget for ZoneBlock {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        block.render(area, buf);
    }
}

impl Button {
    pub fn new(focus: bool, text: String, style: Style) -> Self {
        Self { focus, text, style }
    }
}

impl Widget for Button {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut style = self.style;
        if self.focus {
            style = style.bg(THEME.surface0().term_color());
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .style(style)
            .border_type(BorderType::Rounded);
        let text = Paragraph::new(self.text)
            .style(style)
            .alignment(Alignment::Center);
        let text_area = block.inner(area);
        block.render(area, buf);
        text.render(text_area, buf);
    }
}

impl PathPick {
    pub fn new(path: Validation<PathBuf>) -> Self {
        Self { path }
    }
}

impl StatefulWidget for PathPick {
    type State = TextArea<'static>;

    fn render(self, area: Rect, buf: &mut Buffer, textarea: &mut Self::State) {
        textarea.set_cursor_line_style(Style::default());

        let style = Style::default();

        let color = match &self.path {
            Validation::Valid(_) => THEME.green(),
            Validation::Warning(_, _) => THEME.yellow(),
            Validation::Error(_) => THEME.red(),
        };
        let style = style.fg(color.term_color());

        textarea.set_style(style);
        textarea.set_cursor_style(
            Style::default()
                .fg(THEME.rosewater().term_color())
                .add_modifier(Modifier::REVERSED),
        );
        textarea.widget().render(area, buf);
    }
}

impl<T: Widget + Styled> PromptWidget<T> {
    pub fn new(focus: bool, prompt: String, widget: T) -> Self {
        Self {
            focus,
            prompt,
            widget,
        }
    }
}

impl<T: Widget + Styled<Item = impl Widget>> Widget for PromptWidget<T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut style = self.widget.style();
        if self.focus {
            style = style.bg(THEME.surface0().term_color());
        }
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(19),
                Constraint::Length(2),
                Constraint::Length(10),
                Constraint::Length(2),
            ])
            .split(area);
        let prompt = Paragraph::new(self.prompt).style(
            style
                .add_modifier(Modifier::BOLD)
                .fg(THEME.green().term_color()),
        );
        let widget = self.widget.set_style(style);
        prompt.render(layout[0], buf);
        widget.render(layout[2], buf);
        if self.focus {
            Paragraph::new("◄ ")
                .style(
                    style
                        .add_modifier(Modifier::BOLD)
                        .fg(THEME.rosewater().term_color()),
                )
                .render(layout[1], buf);
            Paragraph::new(" ►")
                .style(
                    style
                        .add_modifier(Modifier::BOLD)
                        .fg(THEME.rosewater().term_color()),
                )
                .render(layout[3], buf);
        }
    }
}

impl<T> PopupWidget<T> {
    fn block(&self) -> Block<'static> {
        ZoneBlock::default()
            .borders(Borders::ALL)
            .title(self.title.clone())
            .title_color(THEME.green().term_color())
            .bottom_text(self.bottom_text.clone())
            .bottom_text_color(self.bottom_text_color)
            .border_type(BorderType::Double)
            .border_color(THEME.lavender().term_color())
            .bg(THEME.surface0().term_color())
            .block()
    }
}

impl<T: Widget> Widget for PopupWidget<T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        let widget_area = block.inner(area);
        Clear.render(area, buf);
        block.render(area, buf);
        self.widget.render(widget_area, buf);
    }
}

impl<T: StatefulWidget> StatefulWidget for PopupWidget<T> {
    type State = T::State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = self.block();
        let widget_area = block.inner(area);
        Clear.render(area, buf);
        block.render(area, buf);
        self.widget.render(widget_area, buf, state);
    }
}
