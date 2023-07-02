use data::user::Nick;
use data::{buffer, client, history, message, Server};
use iced::widget::{column, container, row, vertical_space};
use iced::{Command, Length};

use super::{input_view, scroll_view, user_context};
use crate::theme;
use crate::widget::{selectable_text, Collection, Element};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
}

pub fn view<'a>(
    state: &'a Query,
    status: client::Status,
    history: &'a history::Manager,
    settings: &'a buffer::Settings,
    is_focused: bool,
) -> Element<'a, Message> {
    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Query(&state.server, &state.nick),
            history,
            |message| {
                let message::Source::Query(_, sender) = &message.source else {
                    return None;
                };

                match sender {
                    message::Sender::User(user) => {
                        let timestamp =
                            settings
                                .format_timestamp(&message.server_time)
                                .map(|timestamp| {
                                    selectable_text(timestamp).style(theme::Text::Alpha04)
                                });
                        let nick = user_context::view(
                            selectable_text(settings.nickname.brackets.format(user)).style(
                                theme::Text::Nickname(user.color_seed(&settings.nickname.color)),
                            ),
                            user.clone(),
                        )
                        .map(scroll_view::Message::UserContext);

                        let message = selectable_text(&message.text);

                        Some(
                            container(row![].push_maybe(timestamp).push(nick).push(message)).into(),
                        )
                    }
                    message::Sender::Server => Some(
                        container(selectable_text(&message.text).style(theme::Text::Server)).into(),
                    ),
                    message::Sender::Action => Some(
                        container(selectable_text(&message.text).style(theme::Text::Accent)).into(),
                    ),
                }
            },
        )
        .map(Message::ScrollView),
    )
    .height(Length::Fill);
    let spacing = is_focused.then_some(vertical_space(4));
    let text_input = (is_focused && status.connected()).then(|| {
        input_view::view(
            &state.input_view,
            data::Buffer::Query(state.server.clone(), state.nick.clone()),
        )
        .map(Message::InputView)
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
    input_view: input_view::State,
}

impl Query {
    pub fn new(server: Server, nick: Nick) -> Self {
        Self {
            server,
            nick,
            scroll_view: scroll_view::State::new(),
            input_view: input_view::State::new(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(message);

                let event = event.map(|event| match event {
                    scroll_view::Event::UserContext(event) => Event::UserContext(event),
                });

                (command.map(Message::ScrollView), event)
            }
            Message::InputView(message) => {
                let (command, event) =
                    self.input_view
                        .update(message, &self.server, clients, history);
                let command = command.map(Message::InputView);

                match event {
                    Some(input_view::Event::InputSent) => {
                        let command = Command::batch(vec![
                            command,
                            self.scroll_view.scroll_to_end().map(Message::ScrollView),
                        ]);

                        (command, None)
                    }
                    None => (command, None),
                }
            }
        }
    }

    pub fn focus(&self) -> Command<Message> {
        self.input_view.focus().map(Message::InputView)
    }
}
