use std::path::PathBuf;
use std::time::Duration;

use data::appearance::theme::FontStyle;
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
use crate::widget::{
    Element, color_picker, combo_box, font_style_pick_list, tooltip,
};
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
    FontStyle(Option<FontStyle>),
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
    pub fn open(
        main_window: &Window,
        config: &Config,
    ) -> (Self, Task<window::Id>) {
        let (window, task) = window::open(window::Settings {
            // Just big enough to show all components in combobox
            size: iced::Size::new(555.0, 300.0),
            resizable: false,
            position: main_window
                .position
                .map(|point| {
                    window::Position::Specific(point + Vector::new(20.0, 20.0))
                })
                .unwrap_or_default(),
            exit_on_close_request: false,
            ..window::settings(config)
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
                let font_style = self.component.font_style(&styles).flatten();

                self.component.update(&mut styles, Some(color), font_style);

                *theme = theme
                    .preview(data::Theme::new("Custom Theme".into(), styles));
            }
            Message::FontStyle(font_style) => {
                let mut styles = *theme.styles();
                let color = self.component.color(&styles);

                self.component.update(&mut styles, color, font_style);

                *theme = theme
                    .preview(data::Theme::new("Custom Theme".into(), styles));
            }
            Message::Component(component) => {
                self.hex_input = None;
                self.combo_box = combo_box::State::new(components().collect());

                self.component = component;
            }
            Message::HexInput(input) => {
                let mut styles = *theme.styles();
                let font_style = self.component.font_style(&styles).flatten();

                self.component.update(
                    &mut styles,
                    theme::hex_to_color(&input),
                    font_style,
                );

                *theme = theme
                    .preview(data::Theme::new("Custom Theme".into(), styles));

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
                let original_color = self.component.color(&styles);
                let original_font_style =
                    self.component.font_style(&styles).flatten();

                self.component.update(
                    &mut styles,
                    original_color,
                    original_font_style,
                );

                *theme = theme
                    .preview(data::Theme::new("Custom Theme".into(), styles));
            }
            Message::Clear => {
                self.hex_input = None;

                let mut styles = *theme.styles();

                self.component.update(&mut styles, None, None);

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

        let font_style = self.component.font_style(theme.styles());

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

        let undo = icon(
            icon::undo(),
            if font_style.is_some() {
                "Revert Color & Font Style"
            } else {
                "Revert Color"
            },
            Message::Revert,
            theme,
        );

        let save = match self.save_result {
            Some(is_success) => status_button(is_success),
            None => secondary_button("Save to Disk", Message::Save),
        };
        let apply =
            secondary_button("Apply Colors & Font Styles", Message::Apply);

        let copy = if self.copied {
            success_icon()
        } else {
            icon(icon::copy(), "Copy Theme to URL", Message::Copy, theme)
        };

        let share = icon(
            icon::share(),
            "Share Theme with community",
            Message::Share,
            theme,
        );

        let color_picker = color_picker(color, Message::Color);

        let font_style_pick_list = font_style.map(|font_style| {
            font_style_pick_list(font_style, |font_style_pick| {
                Message::FontStyle(Option::<FontStyle>::from(font_style_pick))
            })
        });

        let content = column![
            row![
                container(component).width(Fill),
                container(hex_input).width(80),
                font_style_pick_list,
                undo,
                copy,
                share,
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
    theme: &'a Theme,
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
        theme,
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

fn secondary_button(label: &str, message: Message) -> Element<'_, Message> {
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

    fn font_style(&self, styles: &Styles) -> Option<Option<FontStyle>> {
        match self {
            Component::General(_) => None,
            Component::Text(text) => Some(text.font_style(&styles.text)),
            Component::Buffer(buffer) => buffer.font_style(&styles.buffer),
            Component::Buttons(_) => None,
        }
    }

    fn update(
        &self,
        styles: &mut Styles,
        color: Option<Color>,
        font_style: Option<FontStyle>,
    ) {
        match self {
            Component::General(general) => {
                if let Some(color) = color {
                    general.update(&mut styles.general, color);
                }
            }
            Component::Text(text) => {
                text.update(&mut styles.text, color, font_style);
            }
            Component::Buffer(buffer) => {
                buffer.update(&mut styles.buffer, color, font_style);
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
    HighlightIndicator,
    HorizontalRule,
    Scrollbar,
    UnreadIndicator,
}

impl General {
    fn color(&self, styles: &theme::General) -> Color {
        match self {
            General::Background => styles.background,
            General::Border => styles.border,
            General::HighlightIndicator => styles
                .highlight_indicator
                .unwrap_or(styles.unread_indicator),
            General::HorizontalRule => styles.horizontal_rule,
            General::Scrollbar => {
                styles.scrollbar.unwrap_or(styles.horizontal_rule)
            }
            General::UnreadIndicator => styles.unread_indicator,
        }
    }

    fn update(&self, styles: &mut theme::General, color: Color) {
        match self {
            General::Background => {
                styles.background = color;
            }
            General::Border => {
                styles.border = color;
            }
            General::HighlightIndicator => {
                styles.highlight_indicator = Some(color);
            }
            General::HorizontalRule => {
                styles.horizontal_rule = color;
            }
            General::Scrollbar => {
                styles.scrollbar = Some(color);
            }
            General::UnreadIndicator => {
                styles.unread_indicator = color;
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
            Text::Warning => styles.warning.color,
            Text::Info => styles.info.color,
            Text::Debug => styles.debug.color,
            Text::Trace => styles.trace.color,
        }
    }

    fn font_style(&self, styles: &theme::Text) -> Option<FontStyle> {
        match self {
            Text::Primary => styles.primary.font_style,
            Text::Secondary => styles.secondary.font_style,
            Text::Tertiary => styles.tertiary.font_style,
            Text::Success => styles.success.font_style,
            Text::Error => styles.error.font_style,
            Text::Warning => styles.warning.font_style,
            Text::Info => styles.info.font_style,
            Text::Debug => styles.debug.font_style,
            Text::Trace => styles.trace.font_style,
        }
    }

    fn update(
        &self,
        styles: &mut theme::Text,
        color: Option<Color>,
        font_style: Option<FontStyle>,
    ) {
        match self {
            Text::Primary => {
                if let Some(color) = color {
                    styles.primary.color = color;
                }
                styles.primary.font_style = font_style;
            }
            Text::Secondary => {
                if let Some(color) = color {
                    styles.secondary.color = color;
                }
                styles.secondary.font_style = font_style;
            }
            Text::Tertiary => {
                if let Some(color) = color {
                    styles.tertiary.color = color;
                }
                styles.tertiary.font_style = font_style;
            }
            Text::Success => {
                if let Some(color) = color {
                    styles.success.color = color;
                }
                styles.success.font_style = font_style;
            }
            Text::Error => {
                if let Some(color) = color {
                    styles.error.color = color;
                }
                styles.error.font_style = font_style;
            }
            Text::Warning => {
                styles.warning.color = color;
                styles.warning.font_style = font_style;
            }
            Text::Info => {
                styles.info.color = color;
                styles.info.font_style = font_style;
            }
            Text::Debug => {
                styles.debug.color = color;
                styles.debug.font_style = font_style;
            }
            Text::Trace => {
                styles.trace.color = color;
                styles.trace.font_style = font_style;
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
    NicknameOffline,
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
            Buffer::NicknameOffline => styles.nickname_offline.color,
            Buffer::Selection => Some(styles.selection),
            Buffer::ServerMessages(server_messages) => {
                server_messages.color(&styles.server_messages)
            }
            Buffer::Timestamp => Some(styles.timestamp.color),
            Buffer::Topic => Some(styles.topic.color),
            Buffer::Url => Some(styles.url.color),
        }
    }

    fn font_style(&self, styles: &theme::Buffer) -> Option<Option<FontStyle>> {
        match self {
            Buffer::Action => Some(styles.action.font_style),
            Buffer::Background => None,
            Buffer::BackgroundTextInput => None,
            Buffer::BackgroundTitleBar => None,
            Buffer::Border => None,
            Buffer::BorderSelected => None,
            Buffer::Code => Some(styles.code.font_style),
            Buffer::Highlight => None,
            Buffer::Nickname => Some(styles.nickname.font_style),
            Buffer::NicknameOffline => Some(styles.nickname_offline.font_style),
            Buffer::Selection => None,
            Buffer::ServerMessages(server_messages) => {
                Some(server_messages.font_style(&styles.server_messages))
            }
            Buffer::Timestamp => Some(styles.timestamp.font_style),
            Buffer::Topic => Some(styles.topic.font_style),
            Buffer::Url => Some(styles.url.font_style),
        }
    }

    fn update(
        &self,
        styles: &mut theme::Buffer,
        color: Option<Color>,
        font_style: Option<FontStyle>,
    ) {
        match self {
            Buffer::Action => {
                if let Some(color) = color {
                    styles.action.color = color;
                }
                styles.action.font_style = font_style;
            }
            Buffer::Background => {
                if let Some(color) = color {
                    styles.background = color;
                }
            }
            Buffer::BackgroundTextInput => {
                if let Some(color) = color {
                    styles.background_text_input = color;
                }
            }
            Buffer::BackgroundTitleBar => {
                if let Some(color) = color {
                    styles.background_title_bar = color;
                }
            }
            Buffer::Border => {
                if let Some(color) = color {
                    styles.border = color;
                }
            }
            Buffer::BorderSelected => {
                if let Some(color) = color {
                    styles.border_selected = color;
                }
            }
            Buffer::Code => {
                if let Some(color) = color {
                    styles.code.color = color;
                }
                styles.code.font_style = font_style;
            }
            Buffer::Highlight => {
                if let Some(color) = color {
                    styles.highlight = color;
                }
            }
            Buffer::Nickname => {
                if let Some(color) = color {
                    styles.nickname.color = color;
                }
                styles.nickname.font_style = font_style;
            }
            Buffer::NicknameOffline => {
                styles.nickname_offline.color = color;
                styles.nickname_offline.font_style = font_style;
            }
            Buffer::Selection => {
                if let Some(color) = color {
                    styles.selection = color;
                }
            }
            Buffer::ServerMessages(server_messages) => {
                server_messages.update(
                    &mut styles.server_messages,
                    color,
                    font_style,
                );
            }
            Buffer::Timestamp => {
                if let Some(color) = color {
                    styles.timestamp.color = color;
                }
                styles.timestamp.font_style = font_style;
            }
            Buffer::Topic => {
                if let Some(color) = color {
                    styles.topic.color = color;
                }
                styles.topic.font_style = font_style;
            }
            Buffer::Url => {
                if let Some(color) = color {
                    styles.url.color = color;
                }
                styles.url.font_style = font_style;
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, strum::Display, strum::EnumIter,
)]
#[strum(serialize_all = "kebab-case")]
pub enum ServerMessages {
    Default,
    #[default]
    Join,
    Part,
    Quit,
    #[strum(serialize = "topic")]
    ReplyTopic,
    ChangeHost,
    ChangeMode,
    ChangeNick,
    ChangeTopic,
    MonitoredOnline,
    MonitoredOffline,
    StandardReplyFail,
    StandardReplyWarn,
    StandardReplyNote,
    #[strum(serialize = "wallops")]
    WAllOps,
    Kick,
}

impl ServerMessages {
    fn color(&self, styles: &theme::ServerMessages) -> Option<Color> {
        match self {
            ServerMessages::Join => styles.join.color,
            ServerMessages::Part => styles.part.color,
            ServerMessages::Quit => styles.quit.color,
            ServerMessages::ReplyTopic => styles.reply_topic.color,
            ServerMessages::ChangeHost => styles.change_host.color,
            ServerMessages::ChangeMode => styles.change_mode.color,
            ServerMessages::ChangeTopic => styles.change_topic.color,
            ServerMessages::ChangeNick => styles.change_nick.color,
            ServerMessages::MonitoredOnline => styles.monitored_online.color,
            ServerMessages::MonitoredOffline => styles.monitored_offline.color,
            ServerMessages::StandardReplyFail => {
                styles.standard_reply_fail.color
            }
            ServerMessages::StandardReplyWarn => {
                styles.standard_reply_warn.color
            }
            ServerMessages::StandardReplyNote => {
                styles.standard_reply_note.color
            }
            ServerMessages::WAllOps => styles.wallops.color,
            ServerMessages::Kick => styles.kick.color,
            ServerMessages::Default => Some(styles.default.color),
        }
    }

    fn font_style(&self, styles: &theme::ServerMessages) -> Option<FontStyle> {
        match self {
            ServerMessages::Join => styles.join.font_style,
            ServerMessages::Part => styles.part.font_style,
            ServerMessages::Quit => styles.quit.font_style,
            ServerMessages::ReplyTopic => styles.reply_topic.font_style,
            ServerMessages::ChangeHost => styles.change_host.font_style,
            ServerMessages::ChangeMode => styles.change_mode.font_style,
            ServerMessages::ChangeNick => styles.change_nick.font_style,
            ServerMessages::ChangeTopic => styles.change_topic.font_style,
            ServerMessages::MonitoredOnline => {
                styles.monitored_online.font_style
            }
            ServerMessages::MonitoredOffline => {
                styles.monitored_offline.font_style
            }
            ServerMessages::StandardReplyFail => {
                styles.standard_reply_fail.font_style
            }
            ServerMessages::StandardReplyWarn => {
                styles.standard_reply_warn.font_style
            }
            ServerMessages::StandardReplyNote => {
                styles.standard_reply_note.font_style
            }
            ServerMessages::WAllOps => styles.wallops.font_style,
            ServerMessages::Kick => styles.kick.font_style,
            ServerMessages::Default => styles.default.font_style,
        }
    }

    fn update(
        &self,
        styles: &mut theme::ServerMessages,
        color: Option<Color>,
        font_style: Option<FontStyle>,
    ) {
        match self {
            ServerMessages::Join => {
                styles.join.color = color;
                styles.join.font_style = font_style;
            }
            ServerMessages::Part => {
                styles.part.color = color;
                styles.part.font_style = font_style;
            }
            ServerMessages::Quit => {
                styles.quit.color = color;
                styles.quit.font_style = font_style;
            }
            ServerMessages::ReplyTopic => {
                styles.reply_topic.color = color;
                styles.reply_topic.font_style = font_style;
            }
            ServerMessages::ChangeHost => {
                styles.change_host.color = color;
                styles.change_host.font_style = font_style;
            }
            ServerMessages::ChangeMode => {
                styles.change_mode.color = color;
                styles.change_mode.font_style = font_style;
            }
            ServerMessages::ChangeNick => {
                styles.change_nick.color = color;
                styles.change_nick.font_style = font_style;
            }
            ServerMessages::ChangeTopic => {
                styles.change_topic.color = color;
                styles.change_topic.font_style = font_style;
            }
            ServerMessages::MonitoredOnline => {
                styles.monitored_online.color = color;
                styles.monitored_online.font_style = font_style;
            }
            ServerMessages::MonitoredOffline => {
                styles.monitored_offline.color = color;
                styles.monitored_offline.font_style = font_style;
            }
            ServerMessages::StandardReplyFail => {
                styles.standard_reply_fail.color = color;
                styles.standard_reply_fail.font_style = font_style;
            }
            ServerMessages::StandardReplyWarn => {
                styles.standard_reply_warn.color = color;
                styles.standard_reply_warn.font_style = font_style;
            }
            ServerMessages::StandardReplyNote => {
                styles.standard_reply_note.color = color;
                styles.standard_reply_note.font_style = font_style;
            }
            ServerMessages::WAllOps => {
                styles.wallops.color = color;
                styles.wallops.font_style = font_style;
            }
            ServerMessages::Kick => {
                styles.kick.color = color;
                styles.kick.font_style = font_style;
            }
            ServerMessages::Default => {
                if let Some(color) = color {
                    styles.default.color = color;
                }
                styles.default.font_style = font_style;
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
                if let Some(color) = color {
                    styles.background = color;
                }
            }
            Button::BackgroundHover => {
                if let Some(color) = color {
                    styles.background_hover = color;
                }
            }
            Button::BackgroundSelected => {
                if let Some(color) = color {
                    styles.background_selected = color;
                }
            }
            Button::BackgroundSelectedHover => {
                if let Some(color) = color {
                    styles.background_selected_hover = color;
                }
            }
        }
    }
}
