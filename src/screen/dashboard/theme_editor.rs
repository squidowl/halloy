use std::path::PathBuf;
use std::time::Duration;

use data::appearance::theme::set_optional_text_style_color;
use data::{Config, url};
use futures::TryFutureExt;
use iced::Length::*;
use iced::alignment::Vertical;
use iced::widget::text::LineHeight;
use iced::widget::{button, center, column, container, row, text_input};
use iced::{Color, Length, Task, Vector, alignment, clipboard};
use strum::IntoEnumIterator;
use tokio::time;

use crate::theme::{self, Styles, Theme};
use crate::widget::{Element, color_picker, combo_box, tooltip};
use crate::window::{self, Window};
use crate::{icon, widget};

#[derive(Debug, Clone)]
pub enum Event {
    Close,
    ReloadThemes,
}

#[derive(Debug, Clone)]
pub enum Message {
    Color(Color),
    Component(Component),
    HexInput(String),
    Save,
    Apply,
    Discard,
    Revert,
    Clear,
    Copy,
    Share,
    SavePath(Option<PathBuf>),
    Saved(Result<(), String>),
    ClearSaveResult,
    ClearCopy,
}

#[derive(Debug, Clone)]
pub struct ThemeEditor {
    pub window: window::Id,
    combo_box: combo_box::State<Component>,
    component: Component,
    hex_input: Option<String>,
    save_result: Option<bool>,
    copied: bool,
}

impl ThemeEditor {
    pub fn open(main_window: &Window) -> (Self, Task<window::Id>) {
        let (window, task) = window::open(window::Settings {
            // Just big enough to show all components in combobox
            size: iced::Size::new(470.0, 300.0),
            resizable: false,
            position: main_window
                .position
                .map(|point| {
                    window::Position::Specific(point + Vector::new(20.0, 20.0))
                })
                .unwrap_or_default(),
            exit_on_close_request: false,
            ..window::settings()
        });

        (
            Self {
                window,
                combo_box: combo_box::State::new(components().collect()),
                // Defaulting to general / background is confusing
                // since picker is same color as background
                component: Component::Text(Text::Primary),
                hex_input: None,
                save_result: None,
                copied: false,
            },
            task,
        )
    }
}

impl ThemeEditor {
    pub fn update(
        &mut self,
        message: Message,
        theme: &mut Theme,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Color(color) => {
                self.hex_input = None;

                let mut styles = *theme.styles();

                self.component.update(&mut styles, Some(color));

                *theme = theme
                    .preview(data::Theme::new("Custom Theme".into(), styles));
            }
            Message::Component(component) => {
                self.hex_input = None;
                self.combo_box = combo_box::State::new(components().collect());

                self.component = component;
            }
            Message::HexInput(input) => {
                if let Some(color) = theme::hex_to_color(&input) {
                    let mut styles = *theme.styles();

                    self.component.update(&mut styles, Some(color));

                    *theme = theme.preview(data::Theme::new(
                        "Custom Theme".into(),
                        styles,
                    ));
                }

                self.hex_input = Some(input);
            }
            Message::Save => {
                let task = async move {
                    rfd::AsyncFileDialog::new()
                        .set_directory(Config::themes_dir())
                        .set_file_name("custom-theme.toml")
                        .save_file()
                        .await
                        .map(|handle| handle.path().to_path_buf())
                };

                return (Task::perform(task, Message::SavePath), None);
            }
            Message::Apply => {
                // Keep theme in preview mode, it'll get overwritten the next time they
                // change theme in-app
                return (Task::none(), Some(Event::Close));
            }
            Message::Discard => {
                // Remove preview to discard it
                *theme = theme.selected();

                return (Task::none(), Some(Event::Close));
            }
            Message::Revert => {
                self.hex_input = None;

                let mut styles = *theme.selected().styles();
                let original = self.component.color(&styles);

                self.component.update(&mut styles, original);

                *theme = theme
                    .preview(data::Theme::new("Custom Theme".into(), styles));
            }
            Message::Clear => {
                self.hex_input = None;

                let mut styles = *theme.styles();

                self.component.update(&mut styles, None);

                *theme = theme
                    .preview(data::Theme::new("Custom Theme".into(), styles));
            }
            Message::Copy => {
                self.copied = true;

                let url = url::theme(theme.styles());

                return (
                    Task::batch(vec![
                        clipboard::write(url),
                        Task::perform(
                            time::sleep(Duration::from_secs(2)),
                            |()| Message::ClearCopy,
                        ),
                    ]),
                    None,
                );
            }
            Message::Share => {
                let url = url::theme_submit(theme.styles());
                let _ = open::that_detached(url);

                return (Task::none(), None);
            }
            Message::SavePath(None) => {}
            Message::SavePath(Some(path)) => {
                log::debug!("Saving theme to {path:?}");

                let styles = *theme.styles();

                return (
                    Task::perform(
                        styles.save(path).map_err(|e| e.to_string()),
                        Message::Saved,
                    ),
                    None,
                );
            }
            Message::Saved(Err(err)) => {
                log::error!("Failed to save theme: {err}");
                self.save_result = Some(false);

                return (
                    Task::perform(time::sleep(Duration::from_secs(2)), |()| {
                        Message::ClearSaveResult
                    }),
                    None,
                );
            }
            Message::Saved(Ok(())) => {
                log::debug!("Theme saved");
                self.save_result = Some(true);

                return (
                    Task::perform(time::sleep(Duration::from_secs(2)), |()| {
                        Message::ClearSaveResult
                    }),
                    Some(Event::ReloadThemes),
                );
            }
            Message::ClearSaveResult => {
                self.save_result = None;
            }
            Message::ClearCopy => {
                self.copied = false;
            }
        }

        (Task::none(), None)
    }

    pub fn view<'a>(&'a self, theme: &'a Theme) -> Element<'a, Message> {
        let color = self
            .component
            .color(theme.styles())
            .unwrap_or(Color::TRANSPARENT);

        let component = combo_box(
            &self.combo_box,
            &self.component.to_string(),
            None,
            Message::Component,
        );

        let is_input_valid = self.hex_input.is_none()
            || self
                .hex_input
                .as_deref()
                .and_then(theme::hex_to_color)
                .is_some();
        let hex_input = text_input(
            "",
            self.hex_input
                .as_deref()
                .unwrap_or(theme::color_to_hex(color).as_str()),
        )
        .on_input(Message::HexInput)
        .style(move |theme, status| {
            if is_input_valid {
                theme::text_input::primary(theme, status)
            } else {
                theme::text_input::error(theme, status)
            }
        });

        let undo = icon(icon::undo(), "Revert Color", Message::Revert);

        let save = match self.save_result {
            Some(is_success) => status_button(is_success),
            None => secondary_button("Save to Disk", Message::Save),
        };
        let apply = secondary_button("Apply Colors", Message::Apply);

        let copy = if self.copied {
            success_icon()
        } else {
            icon(icon::copy(), "Copy Theme to URL", Message::Copy)
        };

        let share =
            icon(icon::share(), "Share Theme with community", Message::Share);

        let color_picker = color_picker(color, Message::Color);

        let content = column![
            row![
                container(component).width(Fill),
                container(hex_input).width(80),
                undo,
                copy,
                share
            ]
            .align_y(Vertical::Center)
            .spacing(4),
            color_picker,
            row![apply, save].spacing(4),
        ]
        .spacing(8);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .style(theme::container::general)
            .into()
    }
}

fn icon<'a>(
    icon: widget::Text<'a>,
    tip: &'a str,
    message: Message,
) -> Element<'a, Message> {
    tooltip(
        button(center(icon.style(theme::text::primary)))
            .width(22)
            .height(22)
            .padding(5)
            .style(|theme, style| theme::button::primary(theme, style, false))
            .on_press(message),
        Some(tip),
        tooltip::Position::Bottom,
    )
}

fn success_icon<'a>() -> Element<'a, Message> {
    button(center(icon::checkmark().style(theme::text::success)))
        .width(22)
        .height(22)
        .padding(5)
        .style(theme::button::bare)
        .into()
}

fn secondary_button(label: &str, message: Message) -> Element<Message> {
    button(
        container(label)
            .align_x(alignment::Horizontal::Center)
            .width(Fill),
    )
    .padding(5)
    .width(Fill)
    .style(|theme, status| theme::button::secondary(theme, status, false))
    .on_press(message)
    .into()
}

fn status_button<'a>(is_success: bool) -> Element<'a, Message> {
    button(
        container(if is_success {
            icon::checkmark().style(theme::text::success)
        } else {
            icon::error().style(theme::text::error)
        })
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .width(Fill)
        .height(LineHeight::default().to_absolute(theme::TEXT_SIZE.into())),
    )
    .padding(5)
    .width(Fill)
    .style(|theme, status| theme::button::secondary(theme, status, false))
    .into()
}

fn components() -> impl Iterator<Item = Component> {
    General::iter()
        .map(Component::General)
        .chain(Text::iter().map(Component::Text))
        .chain(
            Buffer::iter()
                .filter(|buffer| !matches!(buffer, Buffer::ServerMessages(_)))
                .map(Component::Buffer),
        )
        .chain(
            ServerMessages::iter()
                .map(Buffer::ServerMessages)
                .map(Component::Buffer),
        )
        .chain(Button::iter().map(Buttons::Primary).map(Component::Buttons))
        .chain(
            Button::iter()
                .map(Buttons::Secondary)
                .map(Component::Buttons),
        )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
pub enum Component {
    #[strum(to_string = "general-{0}")]
    General(General),
    #[strum(to_string = "text-{0}")]
    Text(Text),
    #[strum(to_string = "buffer-{0}")]
    Buffer(Buffer),
    #[strum(to_string = "button-{0}")]
    Buttons(Buttons),
}

impl Component {
    fn color(&self, styles: &Styles) -> Option<Color> {
        match self {
            Component::General(general) => Some(general.color(&styles.general)),
            Component::Text(text) => text.color(&styles.text),
            Component::Buffer(buffer) => buffer.color(&styles.buffer),
            Component::Buttons(buttons) => Some(buttons.color(&styles.buttons)),
        }
    }

    fn update(&self, styles: &mut Styles, color: Option<Color>) {
        match self {
            Component::General(general) => {
                general.update(&mut styles.general, color);
            }
            Component::Text(text) => text.update(&mut styles.text, color),
            Component::Buffer(buffer) => {
                buffer.update(&mut styles.buffer, color);
            }
            Component::Buttons(buttons) => {
                buttons.update(&mut styles.buttons, color);
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
pub enum General {
    Background,
    Border,
    HorizontalRule,
    UnreadIndicator,
}

impl General {
    fn color(&self, styles: &theme::General) -> Color {
        match self {
            General::Background => styles.background,
            General::Border => styles.border,
            General::HorizontalRule => styles.horizontal_rule,
            General::UnreadIndicator => styles.unread_indicator,
        }
    }

    fn update(&self, styles: &mut theme::General, color: Option<Color>) {
        match self {
            General::Background => {
                styles.background = color.unwrap_or(Color::TRANSPARENT);
            }
            General::Border => {
                styles.border = color.unwrap_or(Color::TRANSPARENT);
            }
            General::HorizontalRule => {
                styles.horizontal_rule = color.unwrap_or(Color::TRANSPARENT);
            }
            General::UnreadIndicator => {
                styles.unread_indicator = color.unwrap_or(Color::TRANSPARENT);
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
pub enum Text {
    Primary,
    Secondary,
    Tertiary,
    Success,
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

impl Text {
    fn color(&self, styles: &theme::Text) -> Option<Color> {
        match self {
            Text::Primary => Some(styles.primary.color),
            Text::Secondary => Some(styles.secondary.color),
            Text::Tertiary => Some(styles.tertiary.color),
            Text::Success => Some(styles.success.color),
            Text::Error => Some(styles.error.color),
            Text::Warning => styles.warning.map(|style| style.color),
            Text::Info => styles.info.map(|style| style.color),
            Text::Debug => styles.debug.map(|style| style.color),
            Text::Trace => styles.trace.map(|style| style.color),
        }
    }

    fn update(&self, styles: &mut theme::Text, color: Option<Color>) {
        match self {
            Text::Primary => {
                styles.primary.color = color.unwrap_or(Color::TRANSPARENT);
            }
            Text::Secondary => {
                styles.secondary.color = color.unwrap_or(Color::TRANSPARENT);
            }
            Text::Tertiary => {
                styles.tertiary.color = color.unwrap_or(Color::TRANSPARENT);
            }
            Text::Success => {
                styles.success.color = color.unwrap_or(Color::TRANSPARENT);
            }
            Text::Error => {
                styles.error.color = color.unwrap_or(Color::TRANSPARENT);
            }
            Text::Warning => {
                set_optional_text_style_color(&mut styles.warning, color);
            }
            Text::Info => {
                set_optional_text_style_color(&mut styles.info, color);
            }
            Text::Debug => {
                set_optional_text_style_color(&mut styles.debug, color);
            }
            Text::Trace => {
                set_optional_text_style_color(&mut styles.trace, color);
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
pub enum Buffer {
    Action,
    Background,
    BackgroundTextInput,
    BackgroundTitleBar,
    Border,
    BorderSelected,
    Code,
    Highlight,
    Nickname,
    Selection,
    #[strum(to_string = "server-message-{0}")]
    ServerMessages(ServerMessages),
    Timestamp,
    Topic,
    Url,
}

impl Buffer {
    fn color(&self, styles: &theme::Buffer) -> Option<Color> {
        match self {
            Buffer::Action => Some(styles.action.color),
            Buffer::Background => Some(styles.background),
            Buffer::BackgroundTextInput => Some(styles.background_text_input),
            Buffer::BackgroundTitleBar => Some(styles.background_title_bar),
            Buffer::Border => Some(styles.border),
            Buffer::BorderSelected => Some(styles.border_selected),
            Buffer::Code => Some(styles.code.color),
            Buffer::Highlight => Some(styles.highlight),
            Buffer::Nickname => Some(styles.nickname.color),
            Buffer::Selection => Some(styles.selection),
            Buffer::ServerMessages(server_messages) => {
                server_messages.color(&styles.server_messages)
            }
            Buffer::Timestamp => Some(styles.timestamp.color),
            Buffer::Topic => Some(styles.topic.color),
            Buffer::Url => Some(styles.url.color),
        }
    }

    fn update(&self, styles: &mut theme::Buffer, color: Option<Color>) {
        match self {
            Buffer::Action => {
                styles.action.color = color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::Background => {
                styles.background = color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::BackgroundTextInput => {
                styles.background_text_input =
                    color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::BackgroundTitleBar => {
                styles.background_title_bar =
                    color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::Border => {
                styles.border = color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::BorderSelected => {
                styles.border_selected = color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::Code => {
                styles.code.color = color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::Highlight => {
                styles.highlight = color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::Nickname => {
                styles.nickname.color = color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::Selection => {
                styles.selection = color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::ServerMessages(server_messages) => {
                server_messages.update(&mut styles.server_messages, color);
            }
            Buffer::Timestamp => {
                styles.timestamp.color = color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::Topic => {
                styles.topic.color = color.unwrap_or(Color::TRANSPARENT);
            }
            Buffer::Url => {
                styles.url.color = color.unwrap_or(Color::TRANSPARENT);
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, strum::Display, strum::EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
pub enum ServerMessages {
    #[default]
    Join,
    Part,
    Quit,
    ReplyTopic,
    ChangeHost,
    MonitoredOnline,
    MonitoredOffline,
    StandardReplyFail,
    StandardReplyWarn,
    StandardReplyNote,
    Wallops,
    ChangeMode,
    ChangeNick,
    Default,
}

impl ServerMessages {
    fn color(&self, styles: &theme::ServerMessages) -> Option<Color> {
        match self {
            ServerMessages::Join => styles.join.map(|style| style.color),
            ServerMessages::Part => styles.part.map(|style| style.color),
            ServerMessages::Quit => styles.quit.map(|style| style.color),
            ServerMessages::ReplyTopic => {
                styles.reply_topic.map(|style| style.color)
            }
            ServerMessages::ChangeHost => {
                styles.change_host.map(|style| style.color)
            }
            ServerMessages::MonitoredOnline => {
                styles.monitored_online.map(|style| style.color)
            }
            ServerMessages::MonitoredOffline => {
                styles.monitored_offline.map(|style| style.color)
            }
            ServerMessages::StandardReplyFail => {
                styles.standard_reply_fail.map(|style| style.color)
            }
            ServerMessages::StandardReplyWarn => {
                styles.standard_reply_warn.map(|style| style.color)
            }
            ServerMessages::StandardReplyNote => {
                styles.standard_reply_note.map(|style| style.color)
            }
            ServerMessages::Wallops => styles.wallops.map(|style| style.color),
            ServerMessages::ChangeMode => {
                styles.change_mode.map(|style| style.color)
            }
            ServerMessages::ChangeNick => {
                styles.change_nick.map(|style| style.color)
            }
            ServerMessages::Default => Some(styles.default.color),
        }
    }

    fn update(&self, styles: &mut theme::ServerMessages, color: Option<Color>) {
        match self {
            ServerMessages::Join => {
                set_optional_text_style_color(&mut styles.join, color);
            }
            ServerMessages::Part => {
                set_optional_text_style_color(&mut styles.part, color);
            }
            ServerMessages::Quit => {
                set_optional_text_style_color(&mut styles.quit, color);
            }
            ServerMessages::ReplyTopic => {
                set_optional_text_style_color(&mut styles.reply_topic, color);
            }
            ServerMessages::ChangeHost => {
                set_optional_text_style_color(&mut styles.change_host, color);
            }
            ServerMessages::MonitoredOnline => set_optional_text_style_color(
                &mut styles.monitored_online,
                color,
            ),
            ServerMessages::MonitoredOffline => {
                set_optional_text_style_color(
                    &mut styles.monitored_offline,
                    color,
                );
            }
            ServerMessages::StandardReplyFail => {
                set_optional_text_style_color(
                    &mut styles.standard_reply_fail,
                    color,
                );
            }
            ServerMessages::StandardReplyWarn => {
                set_optional_text_style_color(
                    &mut styles.standard_reply_warn,
                    color,
                );
            }
            ServerMessages::StandardReplyNote => {
                set_optional_text_style_color(
                    &mut styles.standard_reply_note,
                    color,
                );
            }
            ServerMessages::Wallops => {
                set_optional_text_style_color(&mut styles.wallops, color);
            }
            ServerMessages::ChangeMode => {
                set_optional_text_style_color(&mut styles.change_mode, color);
            }
            ServerMessages::ChangeNick => {
                set_optional_text_style_color(&mut styles.change_nick, color);
            }
            ServerMessages::Default => {
                styles.default.color = color.unwrap_or(Color::TRANSPARENT);
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter,
)]
pub enum Buttons {
    #[strum(to_string = "primary-{0}")]
    Primary(Button),
    #[strum(to_string = "secondary-{0}")]
    Secondary(Button),
}

impl Buttons {
    fn color(&self, styles: &theme::Buttons) -> Color {
        match self {
            Buttons::Primary(button) => button.color(&styles.primary),
            Buttons::Secondary(button) => button.color(&styles.secondary),
        }
    }

    fn update(&self, styles: &mut theme::Buttons, color: Option<Color>) {
        match self {
            Buttons::Primary(button) => {
                button.update(&mut styles.primary, color);
            }
            Buttons::Secondary(button) => {
                button.update(&mut styles.secondary, color);
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, strum::Display, strum::EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
pub enum Button {
    #[default]
    Background,
    BackgroundHover,
    BackgroundSelected,
    BackgroundSelectedHover,
}

impl Button {
    fn color(&self, styles: &theme::Button) -> Color {
        match self {
            Button::Background => styles.background,
            Button::BackgroundHover => styles.background_hover,
            Button::BackgroundSelected => styles.background_selected,
            Button::BackgroundSelectedHover => styles.background_selected_hover,
        }
    }

    fn update(&self, styles: &mut theme::Button, color: Option<Color>) {
        match self {
            Button::Background => {
                styles.background = color.unwrap_or(Color::TRANSPARENT);
            }
            Button::BackgroundHover => {
                styles.background_hover = color.unwrap_or(Color::TRANSPARENT);
            }
            Button::BackgroundSelected => {
                styles.background_selected =
                    color.unwrap_or(Color::TRANSPARENT);
            }
            Button::BackgroundSelectedHover => {
                styles.background_selected_hover =
                    color.unwrap_or(Color::TRANSPARENT);
            }
        }
    }
}
