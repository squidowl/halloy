use data::{client, history, Buffer, Input, Server};
use iced::Command;

use crate::widget::{input, Element};

pub enum Event {
    InputSent,
}

#[derive(Debug, Clone)]
pub enum Message {
    Send(Input),
    CompletionSelected,
}

pub fn view<'a>(state: &State, buffer: Buffer) -> Element<'a, Message> {
    input(
        state.input_id.clone(),
        buffer,
        Message::Send,
        Message::CompletionSelected,
    )
}

#[derive(Debug, Clone)]
pub struct State {
    input_id: input::Id,
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
        server: &Server,
        clients: &mut client::Map,
        history: &mut history::Manager,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::Send(input) => {
                if let Some(encoded) = input.encoded() {
                    clients.send(server, encoded);
                }
                if let Some(message) = clients
                    .nickname(server)
                    .and_then(|nick| input.message(nick))
                {
                    history.record_message(server, message);
                }

                (Command::none(), Some(Event::InputSent))
            }
            Message::CompletionSelected => (input::move_cursor_to_end(self.input_id.clone()), None),
        }
    }

    pub fn focus(&self) -> Command<Message> {
        input::focus(self.input_id.clone())
    }
}
