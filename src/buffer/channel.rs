use std::fmt;

use data::{message::Channel, server::Server};
use iced::{
    pure::{self, column, container, text_input, vertical_space, widget::Column, Element},
    Length,
};

use crate::{
    style,
    theme::Theme,
    widget::{self, sticky_scrollable::scrollable},
};

#[derive(Debug, Clone)]
pub enum Message {
    Send,
    Input(String),
}

pub fn view<'a>(
    state: &State,
    clients: &data::client::Map,
    is_focused: bool,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let messages: Vec<Element<'a, Message>> = clients
        .get_messages(&state.server, &state.channel)
        .into_iter()
        .filter_map(|message| match message.command() {
            data::message::Command::PrivMsg { text, .. } => Some(
                container(
                    pure::text(format!("<{}> {}", message.nickname(), text)).size(style::TEXT_SIZE),
                )
                .into(),
            ),
            _ => None,
        })
        .collect();

    let mut content = column().push(
        container(scrollable(
            Column::with_children(messages).width(Length::Fill),
        ))
        .height(Length::Fill),
    );

    if is_focused {
        content = content.push(vertical_space(Length::Units(5))).push(
            text_input("Send message...", &state.input, Message::Input)
                .on_submit(Message::Send)
                .padding(8)
                .style(style::text_input::primary(theme))
                .size(style::TEXT_SIZE),
        )
    }

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([0, 8])
        .into()
}

#[derive(Debug, Clone)]
pub struct State {
    server: Server,
    channel: Channel,
    input: String,
}

impl State {
    pub fn new(server: Server, channel: Channel) -> Self {
        Self {
            server,
            channel,
            input: String::new(),
        }
    }

    pub fn update(&mut self, message: Message, clients: &data::client::Map) {
        match message {
            Message::Send => {
                clients.send_message(&self.server, &self.channel, &self.input);
                self.input = String::new();
            }
            Message::Input(input) => self.input = input,
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channel = self.channel.to_string();
        let server: String = self.server.clone().into();

        write!(f, "{} ({})", channel, server)
    }
}
