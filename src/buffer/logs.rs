use std::path::PathBuf;

use data::dashboard::BufferAction;
use data::target::Target;
use data::{Config, client, history, isupport, message};
use iced::widget::{container, row};
use iced::{Length, Size, Task};

use super::{scroll_view, user_context};
use crate::widget::{Element, message_content, selectable_text};
use crate::{Theme, font, theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
}

pub enum Event {
    UserContext(user_context::Event),
    OpenBuffer(Target, BufferAction),
    History(Task<history::manager::Message>),
    MarkAsRead,
    OpenUrl(String),
    ImagePreview(PathBuf, url::Url),
}

pub fn view<'a>(
    state: &'a Logs,
    history: &'a history::Manager,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Logs,
            history,
            None,
            None,
            config,
            theme,
            move |message: &'a data::Message, _, _| match message
                .target
                .source()
            {
                message::Source::Internal(message::source::Internal::Logs(
                    level,
                )) => {
                    let timestamp = config
                        .buffer
                        .format_timestamp(&message.server_time)
                        .map(|timestamp| {
                            selectable_text(timestamp)
                                .style(theme::selectable_text::timestamp)
                                .font_maybe(
                                    theme::font_style::timestamp(theme)
                                        .map(font::get),
                                )
                        });

                    let log_level_style = move |message_theme: &Theme| {
                        theme::selectable_text::log_level(message_theme, *level)
                    };
                    let log_level = selectable_text(
                        // Infer left or right alignment preference from
                        // nickname alignment setting
                        if config.buffer.nickname.alignment.is_right() {
                            format!("{level: >5}")
                        } else {
                            format!("{level: <5}")
                        },
                    )
                    .style(log_level_style)
                    .font_maybe(
                        theme::font_style::log_level(theme, *level)
                            .map(font::get),
                    );

                    let message = message_content(
                        &message.content,
                        isupport::DEFAULT_CHANTYPES,
                        isupport::CaseMap::default(),
                        theme,
                        scroll_view::Message::Link,
                        theme::selectable_text::logs,
                        theme::font_style::primary,
                        config,
                    );

                    Some(
                        row![
                            timestamp,
                            log_level,
                            selectable_text(" "),
                            message,
                        ]
                        .into(),
                    )
                }
                _ => None,
            },
        )
        .map(Message::ScrollView),
    )
    .height(Length::Fill);

    container(messages)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone, Default)]
pub struct Logs {
    pub scroll_view: scroll_view::State,
}

impl Logs {
    pub fn new(pane_size: Size) -> Self {
        Self {
            scroll_view: scroll_view::State::new(pane_size),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        history: &history::Manager,
        clients: &client::Map,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(
                    message,
                    false,
                    scroll_view::Kind::Logs,
                    history,
                    clients,
                    config,
                );

                let event = event.and_then(|event| match event {
                    scroll_view::Event::UserContext(event) => {
                        Some(Event::UserContext(event))
                    }
                    scroll_view::Event::OpenBuffer(target, buffer_action) => {
                        Some(Event::OpenBuffer(target, buffer_action))
                    }
                    scroll_view::Event::GoToMessage(_, _, _) => None,
                    scroll_view::Event::RequestOlderChatHistory => None,
                    scroll_view::Event::PreviewChanged => None,
                    scroll_view::Event::HidePreview(..) => None,
                    scroll_view::Event::MarkAsRead => Some(Event::MarkAsRead),
                    scroll_view::Event::OpenUrl(url) => {
                        Some(Event::OpenUrl(url))
                    }
                    scroll_view::Event::ImagePreview(path, url) => {
                        Some(Event::ImagePreview(path, url))
                    }
                });

                (command.map(Message::ScrollView), event)
            }
        }
    }
}
