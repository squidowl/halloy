use std::path::PathBuf;
use std::time::Duration;

use data::{url, Config};
use futures::TryFutureExt;
use iced::alignment::Vertical;
use iced::widget::text::LineHeight;
use iced::widget::{button, center, column, container, row, text_input};
use iced::{alignment, clipboard, Length::*};
use iced::{Color, Length};
use iced::{Task, Vector};
use strum::IntoEnumIterator;
use tokio::time;

use crate::theme::{self, Colors, Theme};
use crate::widget::{color_picker, combo_box, tooltip, Element};
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
                .map(|point| window::Position::Specific(point + Vector::new(20.0, 20.0)))
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

                let mut colors = *theme.colors();

                self.component.update(&mut colors, Some(color));

                *theme = theme.preview(data::Theme::new("Custom Theme".into(), colors));
            }
            Message::Component(component) => {
                self.hex_input = None;
                self.combo_box = combo_box::State::new(components().collect());

                self.component = component
            }
            Message::HexInput(input) => {
                if let Some(color) = theme::hex_to_color(&input) {
                    let mut colors = *theme.colors();

                    self.component.update(&mut colors, Some(color));

                    *theme = theme.preview(data::Theme::new("Custom Theme".into(), colors));
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

                let mut colors = *theme.selected().colors();
                let original = self.component.color(&colors);

                self.component.update(&mut colors, original);

                *theme = theme.preview(data::Theme::new("Custom Theme".into(), colors));
            }
            Message::Clear => {
                self.hex_input = None;

                let mut colors = *theme.colors();

                self.component.update(&mut colors, None);

                *theme = theme.preview(data::Theme::new("Custom Theme".into(), colors));
            }
            Message::Copy => {
                self.copied = true;

                let url = url::theme(theme.colors());

                return (
                    Task::batch(vec![
                        clipboard::write(url),
                        Task::perform(time::sleep(Duration::from_secs(2)), |_| Message::ClearCopy),
                    ]),
                    None,
                );
            }
            Message::SavePath(None) => {}
            Message::SavePath(Some(path)) => {
                log::debug!("Saving theme to {path:?}");

                let colors = *theme.colors();

                return (
                    Task::perform(colors.save(path).map_err(|e| e.to_string()), Message::Saved),
                    None,
                );
            }
            Message::Saved(Err(err)) => {
                log::error!("Failed to save theme: {err}");
                self.save_result = Some(false);

                return (
                    Task::perform(time::sleep(Duration::from_secs(2)), |_| {
                        Message::ClearSaveResult
                    }),
                    None,
                );
            }
            Message::Saved(Ok(_)) => {
                log::debug!("Theme saved");
                self.save_result = Some(true);

                return (
                    Task::perform(time::sleep(Duration::from_secs(2)), |_| {
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
            .color(theme.colors())
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

        let color_picker = color_picker(color, Message::Color);

        let content = column![
            row![
                container(component).width(Fill),
                container(hex_input).width(80),
                undo,
                copy,
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

fn icon<'a>(icon: widget::Text<'a>, tip: &'a str, message: Message) -> Element<'a, Message> {
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
    fn color(&self, colors: &Colors) -> Option<Color> {
        match self {
            Component::General(general) => Some(general.color(&colors.general)),
            Component::Text(text) => Some(text.color(&colors.text)),
            Component::Buffer(buffer) => buffer.color(&colors.buffer),
            Component::Buttons(buttons) => Some(buttons.color(&colors.buttons)),
        }
    }

    fn update(&self, colors: &mut Colors, color: Option<Color>) {
        match self {
            Component::General(general) => general.update(&mut colors.general, color),
            Component::Text(text) => text.update(&mut colors.text, color),
            Component::Buffer(buffer) => buffer.update(&mut colors.buffer, color),
            Component::Buttons(buttons) => buttons.update(&mut colors.buttons, color),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter)]
#[strum(serialize_all = "kebab-case")]
pub enum General {
    Background,
    Border,
    HorizontalRule,
    UnreadIndicator,
}

impl General {
    fn color(&self, colors: &theme::General) -> Color {
        match self {
            General::Background => colors.background,
            General::Border => colors.border,
            General::HorizontalRule => colors.horizontal_rule,
            General::UnreadIndicator => colors.unread_indicator,
        }
    }

    fn update(&self, colors: &mut theme::General, color: Option<Color>) {
        match self {
            General::Background => colors.background = color.unwrap_or(Color::TRANSPARENT),
            General::Border => colors.border = color.unwrap_or(Color::TRANSPARENT),
            General::HorizontalRule => colors.horizontal_rule = color.unwrap_or(Color::TRANSPARENT),
            General::UnreadIndicator => {
                colors.unread_indicator = color.unwrap_or(Color::TRANSPARENT)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter)]
#[strum(serialize_all = "kebab-case")]
pub enum Text {
    Primary,
    Secondary,
    Tertiary,
    Success,
    Error,
}

impl Text {
    fn color(&self, colors: &theme::Text) -> Color {
        match self {
            Text::Primary => colors.primary,
            Text::Secondary => colors.secondary,
            Text::Tertiary => colors.tertiary,
            Text::Success => colors.success,
            Text::Error => colors.error,
        }
    }

    fn update(&self, colors: &mut theme::Text, color: Option<Color>) {
        match self {
            Text::Primary => colors.primary = color.unwrap_or(Color::TRANSPARENT),
            Text::Secondary => colors.secondary = color.unwrap_or(Color::TRANSPARENT),
            Text::Tertiary => colors.tertiary = color.unwrap_or(Color::TRANSPARENT),
            Text::Success => colors.success = color.unwrap_or(Color::TRANSPARENT),
            Text::Error => colors.error = color.unwrap_or(Color::TRANSPARENT),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter)]
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
    fn color(&self, colors: &theme::Buffer) -> Option<Color> {
        match self {
            Buffer::Action => Some(colors.action),
            Buffer::Background => Some(colors.background),
            Buffer::BackgroundTextInput => Some(colors.background_text_input),
            Buffer::BackgroundTitleBar => Some(colors.background_title_bar),
            Buffer::Border => Some(colors.border),
            Buffer::BorderSelected => Some(colors.border_selected),
            Buffer::Code => Some(colors.code),
            Buffer::Highlight => Some(colors.highlight),
            Buffer::Nickname => Some(colors.nickname),
            Buffer::Selection => Some(colors.selection),
            Buffer::ServerMessages(server_messages) => {
                server_messages.color(&colors.server_messages)
            }
            Buffer::Timestamp => Some(colors.timestamp),
            Buffer::Topic => Some(colors.topic),
            Buffer::Url => Some(colors.url),
        }
    }

    fn update(&self, colors: &mut theme::Buffer, color: Option<Color>) {
        match self {
            Buffer::Action => colors.action = color.unwrap_or(Color::TRANSPARENT),
            Buffer::Background => colors.background = color.unwrap_or(Color::TRANSPARENT),
            Buffer::BackgroundTextInput => {
                colors.background_text_input = color.unwrap_or(Color::TRANSPARENT)
            }
            Buffer::BackgroundTitleBar => {
                colors.background_title_bar = color.unwrap_or(Color::TRANSPARENT)
            }
            Buffer::Border => colors.border = color.unwrap_or(Color::TRANSPARENT),
            Buffer::BorderSelected => colors.border_selected = color.unwrap_or(Color::TRANSPARENT),
            Buffer::Code => colors.code = color.unwrap_or(Color::TRANSPARENT),
            Buffer::Highlight => colors.highlight = color.unwrap_or(Color::TRANSPARENT),
            Buffer::Nickname => colors.nickname = color.unwrap_or(Color::TRANSPARENT),
            Buffer::Selection => colors.selection = color.unwrap_or(Color::TRANSPARENT),
            Buffer::ServerMessages(server_messages) => {
                server_messages.update(&mut colors.server_messages, color)
            }
            Buffer::Timestamp => colors.timestamp = color.unwrap_or(Color::TRANSPARENT),
            Buffer::Topic => colors.topic = color.unwrap_or(Color::TRANSPARENT),
            Buffer::Url => colors.url = color.unwrap_or(Color::TRANSPARENT),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, strum::Display, strum::EnumIter)]
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
    Default,
}

impl ServerMessages {
    fn color(&self, colors: &theme::ServerMessages) -> Option<Color> {
        match self {
            ServerMessages::Join => colors.join,
            ServerMessages::Part => colors.part,
            ServerMessages::Quit => colors.quit,
            ServerMessages::ReplyTopic => colors.reply_topic,
            ServerMessages::ChangeHost => colors.change_host,
            ServerMessages::MonitoredOnline => colors.monitored_online,
            ServerMessages::MonitoredOffline => colors.monitored_offline,
            ServerMessages::Default => Some(colors.default),
        }
    }

    fn update(&self, colors: &mut theme::ServerMessages, color: Option<Color>) {
        match self {
            ServerMessages::Join => colors.join = color,
            ServerMessages::Part => colors.part = color,
            ServerMessages::Quit => colors.quit = color,
            ServerMessages::ReplyTopic => colors.reply_topic = color,
            ServerMessages::ChangeHost => colors.change_host = color,
            ServerMessages::MonitoredOnline => colors.monitored_online = color,
            ServerMessages::MonitoredOffline => colors.monitored_offline = color,
            ServerMessages::Default => colors.default = color.unwrap_or(Color::TRANSPARENT),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter)]
pub enum Buttons {
    #[strum(to_string = "primary-{0}")]
    Primary(Button),
    #[strum(to_string = "secondary-{0}")]
    Secondary(Button),
}

impl Buttons {
    fn color(&self, colors: &theme::Buttons) -> Color {
        match self {
            Buttons::Primary(button) => button.color(&colors.primary),
            Buttons::Secondary(button) => button.color(&colors.secondary),
        }
    }

    fn update(&self, colors: &mut theme::Buttons, color: Option<Color>) {
        match self {
            Buttons::Primary(button) => button.update(&mut colors.primary, color),
            Buttons::Secondary(button) => button.update(&mut colors.secondary, color),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, strum::Display, strum::EnumIter)]
#[strum(serialize_all = "kebab-case")]
pub enum Button {
    #[default]
    Background,
    BackgroundHover,
    BackgroundSelected,
    BackgroundSelectedHover,
}

impl Button {
    fn color(&self, colors: &theme::Button) -> Color {
        match self {
            Button::Background => colors.background,
            Button::BackgroundHover => colors.background_hover,
            Button::BackgroundSelected => colors.background_selected,
            Button::BackgroundSelectedHover => colors.background_selected_hover,
        }
    }

    fn update(&self, colors: &mut theme::Button, color: Option<Color>) {
        match self {
            Button::Background => colors.background = color.unwrap_or(Color::TRANSPARENT),
            Button::BackgroundHover => {
                colors.background_hover = color.unwrap_or(Color::TRANSPARENT)
            }
            Button::BackgroundSelected => {
                colors.background_selected = color.unwrap_or(Color::TRANSPARENT)
            }
            Button::BackgroundSelectedHover => {
                colors.background_selected_hover = color.unwrap_or(Color::TRANSPARENT)
            }
        }
    }
}
