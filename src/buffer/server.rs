use data::{history, message, Config};
use iced::widget::{column, container, row, vertical_space};
use iced::{Command, Length};

use super::{input_view, scroll_view};
use crate::theme;
use crate::widget::{selectable_text, Element};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
}

pub fn view<'a>(
    state: &'a Server,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    config: &'a Config,
    is_focused: bool,
) -> Element<'a, Message> {
    let status = clients.status(&state.server);
    let buffer = state.buffer();
    let input = history.input(&buffer);

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Server(&state.server),
            history,
            config,
            |message| {
                let timestamp =
                    config
                        .buffer
                        .format_timestamp(&message.server_time)
                        .map(|timestamp| {
                            selectable_text(timestamp).style(theme::selectable_text::transparent)
                        });

                match message.target.source() {
                    message::Source::Server(_) => {
                        let message =
                            selectable_text(&message.text).style(theme::selectable_text::info);

                        Some(container(row![].push_maybe(timestamp).push(message)).into())
                    }
                    message::Source::Internal(message::source::Internal::Status(status)) => {
                        let message = selectable_text(&message.text)
                            .style(|theme| theme::selectable_text::status(theme, *status));

                        Some(container(row![].push_maybe(timestamp).push(message)).into())
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

    let channels = clients.get_channels(&state.server);

    let text_input = show_text_input.then(|| {
        column![
            vertical_space().height(4),
            input_view::view(
                &state.input_view,
                buffer,
                input,
                &[],
                channels,
                is_focused,
                !status.connected()
            )
            .map(Message::InputView)
        ]
        .width(Length::Fill)
    });

    let scrollable = column![messages]
        .push_maybe(text_input)
        .height(Length::Fill);

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone)]
pub struct Server {
    pub server: data::server::Server,
    pub scroll_view: scroll_view::State,
    pub input_view: input_view::State,
}

impl Server {
    pub fn new(server: data::server::Server) -> Self {
        Self {
            server,
            scroll_view: scroll_view::State::new(),
            input_view: input_view::State::new(),
        }
    }

    pub fn buffer(&self) -> data::Buffer {
        data::Buffer::Server(self.server.clone())
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
    ) -> Command<Message> {
        match message {
            Message::ScrollView(message) => {
                let (command, _) = self.scroll_view.update(message);
                command.map(Message::ScrollView)
            }
            Message::InputView(message) => {
                let (command, event) = self.input_view.update(message, clients, history);
                let command = command.map(Message::InputView);

                match event {
                    Some(input_view::Event::InputSent) => Command::batch(vec![
                        command,
                        self.scroll_view.scroll_to_end().map(Message::ScrollView),
                    ]),
                    None => command,
                }
            }
        }
    }

    pub fn focus(&self) -> Command<Message> {
        self.input_view.focus().map(Message::InputView)
    }

    pub fn reset(&self) -> Command<Message> {
        self.input_view.reset().map(Message::InputView)
    }
}
