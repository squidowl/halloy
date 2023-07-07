use data::{client, history, Config};
use iced::widget::{column, container, row, vertical_space};
use iced::{Command, Length};

use super::{input_view, scroll_view};
use crate::theme;
use crate::widget::{selectable_text, Collection, Element};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
}

pub fn view<'a>(
    state: &'a Server,
    status: client::Status,
    history: &'a history::Manager,
    config: &'a Config,
    is_focused: bool,
) -> Element<'a, Message> {
    let buffer = state.buffer();
    let input_history = history.input_history(&buffer);

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Server(&state.server),
            history,
            |message| {
                let timestamp = settings
                    .format_timestamp(&message.server_time)
                    .map(|timestamp| selectable_text(timestamp).style(theme::Text::Alpha04));

                match message.source {
                    data::message::Source::Server => {
                        let message = selectable_text(&message.text).style(theme::Text::Alpha04);

                        Some(container(row![].push_maybe(timestamp).push(message)).into())
                    }
                    data::message::Source::Status(status) => {
                        let message =
                            selectable_text(&message.text).style(theme::Text::Status(status));

                        Some(container(row![].push_maybe(timestamp).push(message)).into())
                    }
                    _ => None,
                }
            },
        )
        .map(Message::ScrollView),
    )
    .height(Length::Fill);
    let spacing = is_focused.then_some(vertical_space(4));
    let text_input = (is_focused && status.connected()).then(|| {
        input_view::view(&state.input_view, buffer, &[], input_history).map(Message::InputView)
    });

    let scrollable = column![messages]
        .push_maybe(spacing)
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
    input_view: input_view::State,
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
