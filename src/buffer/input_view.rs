use data::user::User;
use data::{client, history, Buffer, Input};
use iced::Command;

use crate::widget::{input, Element};

pub enum Event {
    InputSent,
}

#[derive(Debug, Clone)]
pub enum Message {
    Input(String),
    Send(Input),
    Completion(String),
}

pub fn view<'a>(
    state: &'a State,
    buffer: Buffer,
    users: &'a [User],
    history: &'a [String],
) -> Element<'a, Message> {
    input(
        state.input_id.clone(),
        buffer,
        &state.input,
        users,
        history,
        Message::Input,
        Message::Send,
        Message::Completion,
    )
}

#[derive(Debug, Clone)]
pub struct State {
    input_id: input::Id,
    input: String,
}

impl State {
    pub fn new() -> Self {
        Self {
            input_id: input::Id::unique(),
            input: String::default(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut client::Map,
        history: &mut history::Manager,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::Input(input) => {
                self.input = input;

                (Command::none(), None)
            }
            Message::Send(input) => {
                self.input.clear();

                if let Some(encoded) = input.encoded() {
                    clients.send(input.server(), encoded);
                }

                if let Some(nick) = clients.nickname(input.server()) {
                    history.record_input(input, nick);
                }

                (Command::none(), Some(Event::InputSent))
            }
            Message::Completion(input) => {
                self.input = input;

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
}
