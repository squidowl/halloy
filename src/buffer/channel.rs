use std::fmt;

use data::server::Server;
use data::theme::Theme;
use iced::{
    pure::{
        self, button, column, container, row, text_input, vertical_space, widget::Column, Element,
    },
    Length,
};

use crate::{style, widget::sticky_scrollable::scrollable};

#[derive(Debug, Clone)]
pub enum Message {
    Send,
    Input(String),
    Users,
}

pub fn view<'a>(
    state: &State,
    clients: &data::client::Map,
    is_focused: bool,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let messages: Vec<Element<'a, Message>> = clients
        .get_channel_messages(&state.server, &state.channel)
        .into_iter()
        .filter_map(|message| {
            let nickname = message.nickname().unwrap_or_default();
            let text = message.text()?;

            Some(
                container(pure::text(format!("<{}> {}", nickname, text)).size(style::TEXT_SIZE))
                    .into(),
            )
        })
        .collect();

    let mut messages = column().push(
        container(scrollable(
            Column::with_children(messages)
                .width(Length::Fill)
                .padding([0, 8]),
        ))
        .height(Length::Fill),
    );

    if is_focused {
        messages = messages.push(vertical_space(Length::Units(5))).push(
            text_input("Send message...", &state.input, Message::Input)
                .on_submit(Message::Send)
                .padding(8)
                .style(style::text_input::primary(theme))
                .size(style::TEXT_SIZE),
        )
    }

    let mut content = row();

    // TODO: Maybe we should show it to the right instead of left.
    if state.is_showing_user_list {
        let users = clients.get_channel_users(&state.server, &state.channel);
        let mut column = column().width(Length::Shrink).spacing(1);

        for user in users {
            // TODO: Enable button pushes (interactions) on users.
            column = column.push(
                button(
                    row()
                        .push(pure::text(user.highest_access_level().to_string()))
                        .push(pure::text(user.nickname())),
                )
                .style(style::button::secondary(theme)),
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
    server: Server,
    channel: String,
    input: String,
    is_showing_user_list: bool,
}

impl State {
    pub fn new(server: Server, channel: String) -> Self {
        Self {
            server,
            channel,
            input: String::new(),
            is_showing_user_list: true,
        }
    }

    pub fn update(&mut self, message: Message, clients: &mut data::client::Map) {
        match message {
            Message::Send => {
                clients.send_privmsg(&self.server, &self.channel, &self.input);
                self.input = String::new();
            }
            Message::Input(input) => self.input = input,
            Message::Users => {
                self.is_showing_user_list = !self.is_showing_user_list;
            }
        }
    }

    pub fn channel(&self) -> &str {
        &self.channel
    }

    pub fn server(&self) -> &Server {
        &self.server
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channel = self.channel.to_string();

        write!(f, "{} ({})", channel, self.server)
    }
}
