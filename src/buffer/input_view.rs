use data::input::{Cache, Draft};
use data::user::User;
use data::{client, history, Buffer, Input};
use iced::Command;

use crate::widget::{input, Element};

pub enum Event {
    InputSent,
}

#[derive(Debug, Clone)]
pub enum Message {
    Input(Draft),
    Send(Input),
    Completion(Draft),
}

pub fn view<'a>(
    state: &'a State,
    buffer: Buffer,
    cache: Cache<'a>,
    users: &'a [User],
    channels: &'a [String],
    buffer_focused: bool,
) -> Element<'a, Message> {
    input(
        state.input_id.clone(),
        buffer,
        cache.draft,
        cache.history,
        users,
        channels,
        buffer_focused,
        Message::Input,
        Message::Send,
        Message::Completion,
    )
}

#[derive(Debug, Clone)]
pub struct State {
    input_id: input::Id,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            input_id: input::Id::unique(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut client::Map,
        history: &mut history::Manager,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::Input(draft) => {
                history.record_draft(draft);

                (Command::none(), None)
            }
            Message::Send(input) => {
                if let Some(encoded) = input.encoded() {
                    clients.send(input.buffer(), encoded);
                }

                if let Some(nick) = clients.nickname(input.server()) {
                    history.record_input(input, nick);
                }

                (Command::none(), Some(Event::InputSent))
            }
            Message::Completion(draft) => {
                history.record_draft(draft);

                (input::move_cursor_to_end(self.input_id.clone()), None)
            }
        }
    }

    pub fn focus(&self) -> Command<Message> {
        input::focus(self.input_id.clone())
    }

    pub fn reset(&self) -> Command<Message> {
        input::reset(self.input_id.clone())
    }

    pub fn insert_user(
        &mut self,
        user: User,
        buffer: Buffer,
        history: &mut history::Manager,
    ) -> Command<Message> {
        let mut text = history.input(&buffer).draft.to_string();

        if text.is_empty() {
            text = format!("{}: ", user.nickname());
        } else if text.ends_with(' ') {
            text = format!("{}{}", text, user.nickname());
        } else {
            text = format!("{} {}", text, user.nickname());
        }

        history.record_draft(Draft { buffer, text });

        input::move_cursor_to_end(self.input_id.clone())
    }
}
