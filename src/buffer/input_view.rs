use data::input::{Cache, Draft};
use data::isupport;
use data::user::{Nick, User};
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
    isupport: Vec<&'a isupport::Parameter>,
    buffer_focused: bool,
    disabled: bool,
) -> Element<'a, Message> {
    input(
        state.input_id.clone(),
        buffer,
        cache.draft,
        cache.history,
        users,
        channels,
        isupport,
        buffer_focused,
        disabled,
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
                    let mut user = nick.to_owned().into();

                    // Resolve our attributes if sending this message in a channel
                    if let Buffer::Channel(server, channel) = input.buffer() {
                        if let Some(user_with_attributes) =
                            clients.resolve_user_attributes(server, channel, &user)
                        {
                            user = user_with_attributes.clone();
                        }
                    }

                    history.record_input(input, user);
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
        nick: Nick,
        buffer: Buffer,
        history: &mut history::Manager,
    ) -> Command<Message> {
        let mut text = history.input(&buffer).draft.to_string();

        if text.is_empty() {
            text = format!("{}: ", nick);
        } else if text.ends_with(' ') {
            text = format!("{}{}", text, nick);
        } else {
            text = format!("{} {}", text, nick);
        }

        history.record_draft(Draft { buffer, text });

        input::move_cursor_to_end(self.input_id.clone())
    }
}
