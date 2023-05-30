use std::fmt;

use data::server::Server;
use iced::{
    widget::{column, container, row, scrollable, text, text_input, vertical_space, Column},
    Length,
};

use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    Send,
    Input(String),
}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(
    state: &State,
    clients: &data::client::Map,
    is_focused: bool,
) -> Element<'a, Message> {
    let messages: Vec<Element<'a, Message>> = clients
        .get_channel_messages(&state.server, &state.channel)
        .into_iter()
        .filter_map(|message| {
            let nickname = message.nickname().unwrap_or_default();
            let message = message.text()?;

            Some(container(text(format!("<{}> {}", nickname, message))).into())
        })
        .collect();

    let mut messages = column![container(scrollable(
        Column::with_children(messages)
            .width(Length::Fill)
            .padding([0, 8]),
    ))
    .height(Length::Fill)];

    if is_focused {
        messages = messages.push(vertical_space(5)).push(
            text_input("Send message...", &state.input)
                .on_input(Message::Input)
                .on_submit(Message::Send)
                .padding(8),
        )
    }

    let mut content = row![];

    // TODO: Maybe we should show it to the right instead of left.
    if state.show_users {
        let users = clients.get_channel_users(&state.server, &state.channel);
        let mut column = column![].padding(4).width(Length::Shrink).spacing(1);

        for user in users {
            // TODO: Enable button pushes (interactions) on users.
            column = column.push(
                row![]
                    .padding([0, 4])
                    .push(text(user.highest_access_level().to_string()))
                    .push(text(user.nickname())),
            );
        }

        content = content.push(container(scrollable(column).height(Length::Fill)))
    }

    content = content.push(messages);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[derive(Debug, Clone)]
pub struct State {
    pub server: Server,
    pub channel: String,
    input: String,
    show_users: bool,
}

impl State {
    pub fn new(server: Server, channel: String) -> Self {
        Self {
            server,
            channel,
            input: String::new(),
            show_users: true,
        }
    }

    pub fn update(&mut self, message: Message, clients: &mut data::client::Map) -> Option<Event> {
        match message {
            Message::Send => {
                clients.send_privmsg(&self.server, &self.channel, &self.input);
                self.input = String::new();

                None
            }
            Message::Input(input) => {
                self.input = input;

                None
            }
        }
    }

    pub(crate) fn toggle_show_users(&mut self) {
        self.show_users = !self.show_users;
    }

    pub(crate) fn is_showing_users(&self) -> bool {
        self.show_users
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channel = self.channel.to_string();

        write!(f, "{} ({})", channel, self.server)
    }
}
