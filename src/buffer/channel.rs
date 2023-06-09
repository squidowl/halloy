use std::fmt;

use data::server::Server;
use iced::widget::{column, container, row, scrollable, text, vertical_space, Rule};
use iced::{Command, Length};

use crate::theme;
use crate::widget::{input, Collection, Column, Element};

#[derive(Debug, Clone)]
pub enum Message {
    Send(input::Content),
    CompletionSelected,
}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(
    state: &Channel,
    clients: &data::client::Map,
    is_focused: bool,
) -> Element<'a, Message> {
    let messages: Vec<Element<'a, Message>> = clients
        .get_channel_messages(&state.server, &state.channel)
        .into_iter()
        .filter_map(|message| {
            let user = message.user()?;

            Some(
                container(
                    row![
                        text(format!("<{}>", user.nickname())).style(theme::Text::Nickname(
                            state
                                .unique_user_colors
                                .then(|| user.color_seed().to_string())
                        )),
                        text(&message.text)
                    ]
                    .spacing(4),
                )
                .into(),
            )
        })
        .collect();

    let messages = container(
        scrollable(
            Column::with_children(messages)
                .width(Length::Fill)
                .padding([0, 8]),
        )
        .id(state.scrollable.clone()),
    )
    .height(Length::Fill);

    let spacing = is_focused.then_some(vertical_space(4));
    let text_input = is_focused.then(|| {
        input(
            state.input_id.clone(),
            Message::Send,
            Message::CompletionSelected,
        )
    });

    // TODO: Maybe we should show it to the right instead of left.
    let users = if state.show_users {
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

        let users = container(
            row![
                scrollable(column)
                    .vertical_scroll(
                        iced::widget::scrollable::Properties::new()
                            .width(1)
                            .scroller_width(1)
                    )
                    .style(theme::Scrollable::Hidden),
                Rule::vertical(1)
            ]
            .spacing(4)
            .height(Length::Fill),
        );

        Some(container(users))
    } else {
        None
    };

    let scrollable =
        column![container(row![].push_maybe(users).push(messages)).height(Length::Fill)]
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
pub struct Channel {
    pub server: Server,
    pub channel: String,
    pub topic: Option<String>,
    pub scrollable: scrollable::Id,
    input_id: input::Id,
    show_users: bool,
    unique_user_colors: bool,
}

impl Channel {
    pub fn new(server: Server, channel: String) -> Self {
        Self {
            server,
            channel,
            topic: None,
            scrollable: scrollable::Id::unique(),
            input_id: input::Id::unique(),
            show_users: true,
            unique_user_colors: true,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::Send(content) => {
                match content {
                    input::Content::Text(message) => {
                        clients.send_privmsg(&self.server, &self.channel, &message);
                    }
                    input::Content::Command(command) => {
                        clients.send_command(&self.server, command);
                    }
                }
                return (
                    scrollable::snap_to(self.scrollable.clone(), scrollable::RelativeOffset::END),
                    None,
                );
            }
            Message::CompletionSelected => {
                return (input::move_cursor_to_end(self.input_id.clone()), None);
            }
        }
    }

    pub fn focus(&self) -> Command<Message> {
        input::focus(self.input_id.clone())
    }

    pub(crate) fn toggle_show_users(&mut self) {
        self.show_users = !self.show_users;
    }

    pub(crate) fn toggle_unique_user_colors(&mut self) {
        self.unique_user_colors = !self.unique_user_colors;
    }

    pub(crate) fn is_showing_users(&self) -> bool {
        self.show_users
    }

    pub(crate) fn is_showing_unique_user_colors(&self) -> bool {
        self.unique_user_colors
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channel = self.channel.to_string();

        write!(f, "{} ({})", channel, self.server)
    }
}
