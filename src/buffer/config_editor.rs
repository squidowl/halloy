use std::ops::Range;

use data::Config;
use data::config::buffer::text_input::KeyBindings;
use iced::advanced::text::Highlighter;
use iced::advanced::text::highlighter::Format;
use iced::widget::text::Wrapping;
use iced::widget::{
    self, Space, column, container, operation, row, rule, text, text_editor,
};
use iced::{Font, Length, Task, highlighter, padding};

use crate::appearance::theme;
use crate::widget::{Element, text_editor_key_bindings, tooltip};
use crate::{Theme, font, icon};

#[derive(Debug, Clone)]
pub enum Message {
    Action(text_editor::Action),
    Save,
    Refresh,
    Kill(text_editor_key_bindings::Kill, bool),
    OpenDirectory,
    OpenConfigFile,
    Saved(String, Result<(), String>),
}

pub enum Event {
    ConfigSaved,
}

#[derive(Debug, Clone)]
struct Error {
    /// Single-line summary shown in the footer.
    message: String,
    /// Full rendered error shown on hover, when available.
    details: Option<String>,
    /// Zero-indexed line to mark in the editor, when available.
    line: Option<usize>,
}

impl Error {
    fn message(message: String) -> Self {
        Self {
            message,
            details: None,
            line: None,
        }
    }

    fn parse(error: &data::config::Error) -> Self {
        let data::config::Error::Parse(parse) = error else {
            return Self::message(error.to_string());
        };

        Self {
            message: match parse.line {
                Some(line) => format!("line {}: {}", line + 1, parse.message),
                None => parse.message.clone(),
            },
            details: Some(parse.details.clone()),
            line: parse.line,
        }
    }
}

#[derive(Debug)]
pub struct ConfigEditor {
    id: widget::Id,
    content: text_editor::Content,
    saved_text: String,
    dirty: bool,
    error: Option<Error>,
}

impl Clone for ConfigEditor {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            content: text_editor::Content::with_text(&self.content.text()),
            saved_text: self.saved_text.clone(),
            dirty: self.dirty,
            error: self.error.clone(),
        }
    }
}

impl Default for ConfigEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigEditor {
    pub fn new() -> Self {
        let (text, error) = read_config();

        Self {
            id: widget::Id::unique(),
            content: text_editor::Content::with_text(&text),
            saved_text: text,
            dirty: false,
            error,
        }
    }

    pub fn focus(&self) -> Task<Message> {
        let id = self.id.clone();

        operation::is_focused(id.clone()).then(move |is_focused| {
            if is_focused {
                Task::none()
            } else {
                operation::focus(id.clone())
            }
        })
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.dirty
    }

    fn is_dirty(&self) -> bool {
        self.content.text() != self.saved_text
    }

    pub fn config_reloaded(&mut self, error: Option<&data::config::Error>) {
        self.error = error.map(Error::parse);
    }

    fn move_cursor(&mut self, motion: text_editor::Motion) {
        let action = text_editor::Action::Move(motion);
        self.content.perform(action);
    }

    pub fn scroll_up_page(&mut self) {
        self.move_cursor(text_editor::Motion::PageUp);
    }

    pub fn scroll_down_page(&mut self) {
        self.move_cursor(text_editor::Motion::PageDown);
    }

    pub fn scroll_to_start(&mut self) {
        self.move_cursor(text_editor::Motion::DocumentStart);
    }

    pub fn scroll_to_end(&mut self) {
        self.move_cursor(text_editor::Motion::DocumentEnd);
    }

    pub fn update(
        &mut self,
        message: Message,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Action(action) => {
                let is_edit = action.is_edit();

                if is_edit {
                    self.error = None;
                }

                self.content.perform(action);

                if is_edit {
                    self.dirty = self.is_dirty();
                }

                (Task::none(), None)
            }
            Message::Kill(kill, save_to_clipboard) => {
                let task = text_editor_key_bindings::perform_kill(
                    &mut self.content,
                    kill,
                    save_to_clipboard,
                    config.buffer.text_input.kill_to_clipboard,
                );

                self.dirty = self.is_dirty();
                self.error = None;

                (task, None)
            }
            Message::Refresh => {
                let (text, error) = read_config();

                self.content = text_editor::Content::with_text(&text);
                self.saved_text = text;
                self.dirty = false;
                self.error = error;

                (Task::none(), None)
            }
            Message::Save => {
                let contents = self.content.text();
                let saved_text = contents.clone();

                (
                    Task::perform(
                        async move {
                            tokio::fs::write(Config::path(), contents)
                                .await
                                .map_err(|error| error.to_string())
                        },
                        move |result| Message::Saved(saved_text, result),
                    ),
                    None,
                )
            }
            Message::OpenDirectory => {
                let _ = crate::open_url::open(Config::config_dir());

                (Task::none(), None)
            }
            Message::OpenConfigFile => {
                let _ = crate::open_url::open(Config::path());

                (Task::none(), None)
            }
            Message::Saved(saved_text, Ok(())) => {
                self.saved_text = saved_text;
                self.dirty = self.is_dirty();
                self.error = None;

                (Task::none(), Some(Event::ConfigSaved))
            }
            Message::Saved(_, Err(error)) => {
                self.error = Some(Error::message(error));

                (Task::none(), None)
            }
        }
    }
}

fn read_config() -> (String, Option<Error>) {
    match std::fs::read_to_string(Config::path()) {
        Ok(text) => (text, None),
        Err(error) => (String::new(), Some(Error::message(error.to_string()))),
    }
}

fn current_toml_section(
    content: &text_editor::Content,
    cursor_line: usize,
) -> Option<String> {
    content
        .lines()
        .take(cursor_line + 1)
        .filter_map(|line| {
            let line = line.text.trim();

            (line.starts_with('[')
                && line.ends_with(']')
                && !line.starts_with("[#"))
            .then(|| line.to_owned())
        })
        .last()
}

pub fn view<'a>(
    state: &'a ConfigEditor,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let cursor = state.content.cursor();
    let position = text(format!(
        "{}:{}",
        cursor.position.line + 1,
        cursor.position.column + 1
    ))
    .style(theme::text::secondary)
    .font_maybe(theme::font_style::secondary(theme).map(font::get));

    let error_or_section = if let Some(error) = &state.error {
        tooltip(
            text(error.message.as_str())
                .style(theme::text::error)
                .font_maybe(theme::font_style::error(theme).map(font::get))
                .wrapping(Wrapping::None)
                .ellipsis(text::Ellipsis::End),
            error.details.as_deref(),
            tooltip::Position::Top,
            theme,
        )
    } else {
        let section =
            current_toml_section(&state.content, cursor.position.line);

        text(section.unwrap_or_default())
            .style(theme::text::secondary)
            .font_maybe(theme::font_style::secondary(theme).map(font::get))
            .wrapping(Wrapping::None)
            .ellipsis(text::Ellipsis::End)
            .into()
    };

    let mut info =
        row![position, error_or_section, Space::new().width(Length::Fill)]
            .spacing(8)
            .padding(padding::bottom(6))
            .align_y(iced::Alignment::Center);

    let dirty_indicator: Element<'a, Message> = if state.dirty {
        tooltip(
            icon::dot().style(theme::text::tertiary).size(8),
            Some("Unsaved changes"),
            tooltip::Position::Top,
            theme,
        )
    } else {
        Space::new().into()
    };

    info = info.push(
        container(dirty_indicator)
            .width(12)
            .align_x(iced::Alignment::Center),
    );

    let footer = container(
        column![container(rule::horizontal(1)).width(Length::Fill), info]
            .spacing(6),
    )
    .padding(padding::horizontal(4))
    .width(Length::Fill);

    let editor = text_editor(&state.content)
        .id(state.id.clone())
        .padding(8)
        .height(Length::Fill)
        .font(font::MONO.clone())
        .style(theme::text_editor::primary)
        .on_action(Message::Action)
        .key_binding(move |key_press| {
            if !matches!(key_press.status, text_editor::Status::Focused { .. })
            {
                return None;
            }

            if matches!(
                config.buffer.text_input.key_bindings,
                KeyBindings::Emacs
            ) && let Some(binding) =
                text_editor_key_bindings::emacs(&key_press, |kill| {
                    text_editor::Binding::Custom(Message::Kill(kill, true))
                })
            {
                return Some(binding);
            }

            if let Some(binding) = text_editor_key_bindings::platform_kill(
                &key_press,
                state.content.selection().is_some(),
                |kill| text_editor::Binding::Custom(Message::Kill(kill, false)),
            ) {
                return Some(binding);
            }

            text_editor::Binding::from_key_press(key_press)
        })
        .highlight_with::<ConfigHighlighter>(
            Settings {
                highlighter: highlighter::Settings {
                    theme: syntax_theme(theme),
                    token: "toml".to_owned(),
                },
                error_line: state.error.as_ref().and_then(|error| error.line),
            },
            token_format,
        );

    let content = column![editor, footer].spacing(1).padding([2, 2]);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Picks a syntect color scheme matching the active theme's light/dark mode.
fn syntax_theme(theme: &Theme) -> highlighter::Theme {
    let background =
        data::appearance::theme::to_hsl(theme.styles().general.background);

    if background.lightness < 0.5 {
        highlighter::Theme::SolarizedDark
    } else {
        highlighter::Theme::InspiredGitHub
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Settings {
    highlighter: highlighter::Settings,
    /// Zero-indexed line of a config parse error, marked as an error.
    error_line: Option<usize>,
}

enum Highlight {
    Syntax(highlighter::Highlight),
    Error,
}

fn token_format(highlight: &Highlight, theme: &Theme) -> Format<Font> {
    match highlight {
        Highlight::Syntax(highlight) => highlight.to_format(),
        Highlight::Error => Format {
            color: Some(theme.styles().text.error.color),
            font: None,
        },
    }
}

// iced toml highlighter, with the config error lines.
struct ConfigHighlighter {
    inner: highlighter::Highlighter,
    error_line: Option<usize>,
}

impl Highlighter for ConfigHighlighter {
    type Settings = Settings;
    type Highlight = Highlight;
    type Iterator<'a> =
        Box<dyn Iterator<Item = (Range<usize>, Highlight)> + 'a>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            inner: highlighter::Highlighter::new(&settings.highlighter),
            error_line: settings.error_line,
        }
    }

    fn update(&mut self, settings: &Self::Settings) {
        self.inner.update(&settings.highlighter);
        self.error_line = settings.error_line;
    }

    fn change_line(&mut self, line: usize) {
        self.inner.change_line(line);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        if Some(self.inner.current_line()) == self.error_line {
            self.inner.highlight_line(line).for_each(drop);

            Box::new(std::iter::once((0..line.len(), Highlight::Error)))
        } else {
            Box::new(self.inner.highlight_line(line).map(
                |(range, highlight)| (range, Highlight::Syntax(highlight)),
            ))
        }
    }

    fn current_line(&self) -> usize {
        self.inner.current_line()
    }
}
