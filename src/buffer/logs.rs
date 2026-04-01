use std::path::PathBuf;

use chrono::{DateTime, Utc};
use data::{Config, Preview, client, history, message};
use iced::widget::{container, row};
use iced::{Length, Size, Task};

use super::{context_menu, scroll_view};
use crate::widget::{Element, selectable_text};
use crate::{Theme, font, theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
}

pub enum Event {
    ContextMenu(context_menu::Event),
    History(Task<history::manager::Message>),
    MarkAsRead,
    OpenUrl(String),
    ImagePreview(PathBuf, url::Url),
    ExpandCondensedMessage(DateTime<Utc>, message::Hash),
    ContractCondensedMessage(DateTime<Utc>, message::Hash),
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
            Option::<fn(&Preview, &message::Source) -> bool>::None,
            None,
            config,
            theme,
            move |message: &'a data::Message, _, _, _, _| match message
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
                            context_menu::timestamp(
                                selectable_text(timestamp)
                                    .style(theme::selectable_text::timestamp)
                                    .font_maybe(
                                        theme::font_style::timestamp(theme)
                                            .map(font::get),
                                    ),
                                &message.server_time,
                                config,
                                theme,
                            )
                            .map(scroll_view::Message::ContextMenu)
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

                    let message = selectable_text(message.text())
                        .font_maybe(
                            theme::font_style::primary(theme).map(font::get),
                        )
                        .style(theme::selectable_text::logs);

                    Some(
                        row![
                            timestamp,
                            selectable_text(" "),
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

#[derive(Debug, Clone)]
pub struct Logs {
    pub scroll_view: scroll_view::State,
}

impl Logs {
    pub fn new(pane_size: Size, config: &Config) -> Self {
        Self {
            scroll_view: scroll_view::State::new(pane_size, config),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        history: &mut history::Manager,
        clients: &mut client::Map,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(
                    message,
                    false,
                    scroll_view::Kind::Logs,
                    None,
                    history,
                    clients,
                    config,
                );

                let event = event.and_then(|event| match event {
                    scroll_view::Event::ContextMenu(event) => {
                        Some(Event::ContextMenu(event))
                    }
                    scroll_view::Event::OpenBuffer(_, _, _) => None,
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
                    scroll_view::Event::ExpandCondensedMessage(
                        server_time,
                        hash,
                    ) => Some(Event::ExpandCondensedMessage(server_time, hash)),
                    scroll_view::Event::ContractCondensedMessage(
                        server_time,
                        hash,
                    ) => {
                        Some(Event::ContractCondensedMessage(server_time, hash))
                    }
                });

                (command.map(Message::ScrollView), event)
            }
        }
    }
}
