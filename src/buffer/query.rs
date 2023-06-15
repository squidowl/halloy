use core::fmt;

use data::{history, Server, User};
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
    user_colors: &'a data::config::UserColor,
    is_focused: bool,
) -> Element<'a, Message> {
    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Query(&state.server, &state.user),
            history,
            |message| {
                let user = message.user()?;

                Some(
                    container(row![
                        selectable_text(format!("<{}> ", user.nickname()))
                            .style(theme::Text::Nickname(user.color_seed(user_colors))),
                        selectable_text(&message.text)
                    ])
                    .into(),
                )
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
    pub user: User,
    pub scroll_view: scroll_view::State,
    input_id: input::Id,
}

impl Query {
    pub fn new(server: Server, user: User) -> Self {
        Self {
            server,
            user,
            scroll_view: scroll_view::State::new(),
            input_id: input::Id::unique(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::Send(content) => {
                if let input::Content::Command(command) = content {
                    clients.send_command(&self.server, command);
                    (
                        self.scroll_view.scroll_to_end().map(Message::ScrollView),
                        None,
                    )
                } else {
                    (Command::none(), None)
                }
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

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user.formatted())
    }
}
