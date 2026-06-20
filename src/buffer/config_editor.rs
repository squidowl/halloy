use std::ops::Range;

use data::Config;
use data::config::buffer::text_input::KeyBindings;
use iced::advanced::text::Highlighter;
use iced::advanced::text::highlighter::Format;
use iced::widget::{column, container, row, rule, text, text_editor};
use iced::{Font, Length, Task, highlighter, padding};

use crate::appearance::theme;
use crate::widget::editor_history::History;
use crate::widget::{Element, text_editor_key_bindings, tooltip};
use crate::{Theme, font, icon};

#[derive(Debug, Clone)]
pub enum Message {
    Action(text_editor::Action),
    Save,
    Refresh,
    Undo,
    Redo,
    Kill(text_editor_key_bindings::Kill, bool),
    OpenDirectory,
    Saved(Result<(), String>),
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
    content: text_editor::Content,
    dirty: bool,
    error: Option<Error>,
    history: History,
}

impl Clone for ConfigEditor {
    fn clone(&self) -> Self {
        Self {
            content: text_editor::Content::with_text(&self.content.text()),
            dirty: self.dirty,
            error: self.error.clone(),
            history: self.history.clone(),
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
            content: text_editor::Content::with_text(&text),
            dirty: false,
            error,
            history: History::new(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn config_reloaded(&mut self, error: Option<&data::config::Error>) {
        self.error = error.map(Error::parse);
    }

    pub fn update(
        &mut self,
        message: Message,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Action(action) => {
                self.history.track(&self.content, &action);

                if action.is_edit() {
                    self.dirty = true;
                    self.error = None;
                }

                self.content.perform(action);

                (Task::none(), None)
            }
            Message::Undo => {
                if self.history.undo(&mut self.content) {
                    self.dirty = true;
                    self.error = None;
                }

                (Task::none(), None)
            }
            Message::Redo => {
                if self.history.redo(&mut self.content) {
                    self.dirty = true;
                    self.error = None;
                }

                (Task::none(), None)
            }
            Message::Kill(kill, save_to_clipboard) => {
                let task = text_editor_key_bindings::perform_kill(
                    &mut self.content,
                    &mut self.history,
                    kill,
                    save_to_clipboard,
                    config.buffer.text_input.kill_to_clipboard,
                );

                self.dirty = true;
                self.error = None;

                (task, None)
            }
            Message::Refresh => {
                let (text, error) = read_config();

                self.content = text_editor::Content::with_text(&text);
                self.dirty = false;
                self.error = error;
                self.history.clear();

                (Task::none(), None)
            }
            Message::Save => {
                let contents = self.content.text();

                (
                    Task::perform(
                        async move {
                            tokio::fs::write(Config::path(), contents)
                                .await
                                .map_err(|error| error.to_string())
                        },
                        Message::Saved,
                    ),
                    None,
                )
            }
            Message::OpenDirectory => {
                let _ = crate::open_url::open(Config::config_dir());

                (Task::none(), None)
            }
            Message::Saved(Ok(())) => {
                self.dirty = false;
                self.error = None;

                (Task::none(), Some(Event::ConfigSaved))
            }
            Message::Saved(Err(error)) => {
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

    let mut info = row![container(position).width(Length::Fill)]
        .spacing(8)
        .padding(padding::bottom(6))
        .align_y(iced::Alignment::Center);

    if let Some(error) = &state.error {
        info = info.push(tooltip(
            text(error.message.as_str())
                .style(theme::text::error)
                .font_maybe(theme::font_style::error(theme).map(font::get)),
            error.details.as_deref(),
            tooltip::Position::Top,
            theme,
        ));
    }

    if state.dirty {
        info = info.push(tooltip(
            container(icon::dot().style(theme::text::tertiary).size(8))
                .padding(padding::right(4)),
            Some("Unsaved changes"),
            tooltip::Position::Top,
            theme,
        ));
    }

    let footer = container(
        column![container(rule::horizontal(1)).width(Length::Fill), info]
            .spacing(6),
    )
    .padding(padding::horizontal(4))
    .width(Length::Fill);

    let editor = text_editor(&state.content)
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

            match key_press.key.as_ref() {
                iced::keyboard::Key::Character("z")
                    if key_press.modifiers.command()
                        && key_press.modifiers.shift() =>
                {
                    Some(text_editor::Binding::Custom(Message::Redo))
                }
                iced::keyboard::Key::Character("z")
                    if key_press.modifiers.command() =>
                {
                    Some(text_editor::Binding::Custom(Message::Undo))
                }
                iced::keyboard::Key::Character("y")
                    if key_press.modifiers.command() =>
                {
                    Some(text_editor::Binding::Custom(Message::Redo))
                }
                _ => text_editor::Binding::from_key_press(key_press),
            }
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
