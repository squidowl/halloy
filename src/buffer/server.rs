use std::path::PathBuf;

use data::dashboard::BufferAction;
use data::target::Target;
use data::user::Nick;
use data::{Config, User, buffer, history, message};
use iced::widget::{column, container, row, vertical_space};
use iced::{Color, Length, Size, Task};

use super::{context_menu, input_view, scroll_view};
use crate::widget::{Element, message_content, selectable_text};
use crate::{Theme, font, theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
}

pub enum Event {
    ContextMenu(context_menu::Event),
    OpenBuffers(Vec<(Target, BufferAction)>),
    LeaveBuffers(Vec<Target>, Option<String>),
    History(Task<history::manager::Message>),
    MarkAsRead(history::Kind),
    OpenUrl(String),
    ImagePreview(PathBuf, url::Url),
}

pub fn view<'a>(
    state: &'a Server,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    config: &'a Config,
    theme: &'a Theme,
    is_focused: bool,
) -> Element<'a, Message> {
    let status = clients.status(&state.server);
    let chantypes = clients.get_chantypes(&state.server);
    let casemapping = clients.get_casemapping(&state.server);
    let buffer = &state.buffer;
    let input = history.input(buffer);
    let our_nick: Option<data::user::NickRef<'_>> =
        clients.nickname(&state.server);
    let our_user = our_nick.map(|our_nick| User::from(Nick::from(our_nick)));

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Server(&state.server),
            history,
            None,
            None,
            config,
            theme,
            move |message: &'a data::Message, _, _, _| {
                let timestamp = config
                    .buffer
                    .format_timestamp(&message.server_time)
                    .map(|timestamp| {
                        context_menu::timestamp(
                            selectable_text(timestamp)
                                .font_maybe(
                                    theme::font_style::timestamp(theme)
                                        .map(font::get),
                                )
                                .style(theme::selectable_text::timestamp),
                            &message.server_time,
                            config,
                            theme,
                        )
                        .map(scroll_view::Message::ContextMenu)
                    });

                match message.target.source() {
                    message::Source::Server(server) => {
                        let message = message_content(
                            &message.content,
                            chantypes,
                            casemapping,
                            theme,
                            scroll_view::Message::Link,
                            move |theme| {
                                theme::selectable_text::server(
                                    theme,
                                    server.as_ref(),
                                )
                            },
                            move |theme| {
                                theme::font_style::server(
                                    theme,
                                    server.as_ref(),
                                )
                            },
                            Option::<fn(Color) -> Color>::None,
                            config,
                        );

                        Some(
                            container(row![
                                timestamp,
                                selectable_text(" "),
                                message
                            ])
                            .into(),
                        )
                    }
                    message::Source::Internal(
                        message::source::Internal::Status(status),
                    ) => {
                        let message = message_content(
                            &message.content,
                            chantypes,
                            casemapping,
                            theme,
                            scroll_view::Message::Link,
                            move |theme| {
                                theme::selectable_text::status(theme, *status)
                            },
                            move |theme| {
                                theme::font_style::status(theme, *status)
                            },
                            Option::<fn(Color) -> Color>::None,
                            config,
                        );

                        Some(
                            container(row![
                                timestamp,
                                selectable_text(" "),
                                message
                            ])
                            .into(),
                        )
                    }
                    _ => None,
                }
            },
        )
        .map(Message::ScrollView),
    )
    .height(Length::Fill);

    let show_text_input = match config.buffer.text_input.visibility {
        data::buffer::TextInputVisibility::Focused => is_focused,
        data::buffer::TextInputVisibility::Always => true,
    };

    let text_input = show_text_input.then(|| {
        column![
            vertical_space().height(4),
            input_view::view(
                &state.input_view,
                input,
                is_focused,
                our_user.as_ref(),
                !status.connected(),
                config,
                theme,
            )
            .map(Message::InputView)
        ]
        .width(Length::Fill)
    });

    let scrollable = column![messages, text_input,].height(Length::Fill);

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone)]
pub struct Server {
    pub buffer: buffer::Upstream,
    pub server: data::server::Server,
    pub scroll_view: scroll_view::State,
    pub input_view: input_view::State,
}

impl Server {
    pub fn new(
        server: data::server::Server,
        pane_size: Size,
        config: &Config,
    ) -> Self {
        Self {
            buffer: buffer::Upstream::Server(server.clone()),
            server,
            scroll_view: scroll_view::State::new(pane_size, config),
            input_view: input_view::State::new(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(
                    message,
                    false,
                    scroll_view::Kind::Server(&self.server),
                    history,
                    clients,
                    config,
                );

                let event = event.and_then(|event| match event {
                    scroll_view::Event::ContextMenu(event) => {
                        Some(Event::ContextMenu(event))
                    }
                    scroll_view::Event::OpenBuffer(target, buffer_action) => {
                        Some(Event::OpenBuffers(vec![(target, buffer_action)]))
                    }
                    scroll_view::Event::GoToMessage(_, _, _) => None,
                    scroll_view::Event::RequestOlderChatHistory => None,
                    scroll_view::Event::PreviewChanged => None,
                    scroll_view::Event::HidePreview(..) => None,
                    scroll_view::Event::MarkAsRead => {
                        history::Kind::from_buffer(data::Buffer::Upstream(
                            self.buffer.clone(),
                        ))
                        .map(Event::MarkAsRead)
                    }
                    scroll_view::Event::OpenUrl(url) => {
                        Some(Event::OpenUrl(url))
                    }
                    scroll_view::Event::ImagePreview(path, url) => {
                        Some(Event::ImagePreview(path, url))
                    }
                });

                (command.map(Message::ScrollView), event)
            }
            Message::InputView(message) => {
                let (command, event) = self.input_view.update(
                    message,
                    &self.buffer,
                    clients,
                    history,
                    config,
                );
                let command = command.map(Message::InputView);

                match event {
                    Some(input_view::Event::InputSent { history_task }) => (
                        Task::batch(vec![
                            command,
                            self.scroll_view
                                .scroll_to_end(config)
                                .map(Message::ScrollView),
                        ]),
                        Some(Event::History(history_task)),
                    ),
                    Some(input_view::Event::OpenBuffers { targets }) => {
                        (command, Some(Event::OpenBuffers(targets)))
                    }
                    Some(input_view::Event::LeaveBuffers {
                        targets,
                        reason,
                    }) => (command, Some(Event::LeaveBuffers(targets, reason))),
                    Some(input_view::Event::Cleared { history_task }) => {
                        (command, Some(Event::History(history_task)))
                    }
                    None => (command, None),
                }
            }
        }
    }

    pub fn focus(&self) -> Task<Message> {
        self.input_view.focus().map(Message::InputView)
    }

    pub fn reset(&mut self) {
        self.input_view.reset();
    }
}
