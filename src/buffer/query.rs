use core::fmt;

use data::user::Nick;
use data::{history, Server};
use iced::widget::{column, container, row, vertical_space};
use iced::{Command, Length};

use super::scroll_view;
use crate::theme;
use crate::widget::{input, selectable_text, Collection, Element};

#[derive(Debug, Clone)]
pub enum Message {
    Send(input::Content),
    CompletionSelected,
    ScrollView(scroll_view::Message),
}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(
    state: &'a Query,
    history: &'a history::Manager,
    buffer_config: &'a data::config::Buffer,
    is_focused: bool,
) -> Element<'a, Message> {
    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Query(&state.server, &state.nick),
            history,
            |message| {
                let user = message.sent_by()?;
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

                Some(container(row![].push_maybe(timestamp).push(nick).push(message)).into())
            },
        )
        .map(Message::ScrollView),
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
pub struct Query {
    pub server: Server,
    pub nick: Nick,
    pub scroll_view: scroll_view::State,
    input_id: input::Id,
}

impl Query {
    pub fn new(server: Server, nick: Nick) -> Self {
        Self {
            server,
            nick,
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
                            clients.send_user_message(&self.server, &self.nick, &message)
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

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.nick)
    }
}
