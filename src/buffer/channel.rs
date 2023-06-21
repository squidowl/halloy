use std::fmt;

use data::history;
use data::server::Server;
use iced::widget::{column, container, row, scrollable, text, vertical_space};
use iced::{Command, Length};

use super::scroll_view;
use crate::theme;
use crate::widget::{input, selectable_text, Collection, Column, Element};

#[derive(Debug, Clone)]
pub enum Message {
    Send(input::Content),
    CompletionSelected,
    ScrollView(scroll_view::Message),
}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(
    state: &'a Channel,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    channel_config: &data::channel::Config,
    buffer_config: &'a data::config::Buffer,
    is_focused: bool,
) -> Element<'a, Message> {
    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Channel(&state.server, &state.channel),
            history,
            |message| match &message.source {
                data::message::Source::Channel(_, kind) => match kind {
                    data::message::ChannelSender::User(user) => {
                        let timestamp = buffer_config.timestamp.clone().map(|timestamp| {
                            let content = &message.formatted_datetime(timestamp.format.as_str());
                            selectable_text(content_with_brackets(content, &timestamp.brackets))
                                .style(theme::Text::Alpha04)
                        });
                        let nick = selectable_text(content_with_brackets(
                            user,
                            &buffer_config.nickname.brackets,
                        ))
                        .style(theme::Text::Nickname(
                            user.color_seed(&buffer_config.nickname.color),
                        ));
                        let message = selectable_text(&message.text);

                        Some(
                            container(row![].push_maybe(timestamp).push(nick).push(message)).into(),
                        )
                    }
                    data::message::ChannelSender::Server => Some(
                        container(selectable_text(&message.text).style(theme::Text::Server)).into(),
                    ),
                },
                _ => None,
            },
        )
        .map(Message::ScrollView),
    )
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    let spacing = is_focused.then_some(vertical_space(4));
    let text_input = is_focused.then(|| {
        input(
            state.input_id.clone(),
            Message::Send,
            Message::CompletionSelected,
        )
    });

    let user_column = {
        let users = clients.get_channel_users(&state.server, &state.channel);
        let column = Column::with_children(
            users
                .iter()
                .map(|user| {
                    container(row![].padding([0, 4]).push(text(format!(
                        "{}{}",
                        user.highest_access_level(),
                        user.nickname()
                    ))))
                    .into()
                })
                .collect(),
        )
        .padding(4)
        .spacing(1);

        container(
            scrollable(column)
                .vertical_scroll(
                    iced::widget::scrollable::Properties::new()
                        .width(1)
                        .scroller_width(1),
                )
                .style(theme::Scrollable::Hidden),
        )
        .width(Length::Shrink)
        .max_width(120)
        .height(Length::Fill)
    };

    let content = match (channel_config.users.visible, channel_config.users.position) {
        (true, data::channel::Position::Left) => {
            row![user_column, messages]
        }
        (true, data::channel::Position::Right) => {
            row![messages, user_column]
        }
        (false, _) => { row![messages] }.height(Length::Fill),
    };

    let scrollable = column![container(content).height(Length::Fill)]
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
    pub scroll_view: scroll_view::State,
    input_id: input::Id,
}

impl Channel {
    pub fn new(server: Server, channel: String) -> Self {
        Self {
            server,
            channel,
            topic: None,
            scroll_view: scroll_view::State::new(),
            input_id: input::Id::unique(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::Send(content) => {
                match content {
                    input::Content::Text(message) => {
                        if let Some(message) =
                            clients.send_channel_message(&self.server, &self.channel, &message)
                        {
                            history.add_message(&self.server, message);
                        }
                    }
                    input::Content::Command(command) => {
                        if let Some(message) = clients.send_command(&self.server, command) {
                            history.add_message(&self.server, message);
                        }
                    }
                }

                (
                    self.scroll_view.scroll_to_end().map(Message::ScrollView),
                    None,
                )
            }
            Message::CompletionSelected => (input::move_cursor_to_end(self.input_id.clone()), None),
            Message::ScrollView(message) => {
                let command = self.scroll_view.update(message);
                (command.map(Message::ScrollView), None)
            }
        }
    }

    pub fn focus(&self) -> Command<Message> {
        input::focus(self.input_id.clone())
    }
}

fn content_with_brackets(
    content: impl std::fmt::Display,
    brackets: &data::config::Brackets,
) -> String {
    format!("{}{}{} ", brackets.left, content, brackets.right)
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let channel = self.channel.to_string();

        write!(f, "{} ({})", channel, self.server)
    }
}
