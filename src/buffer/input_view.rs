use data::input::{self, Cache, Draft};
use data::user::Nick;
use data::{client, history, Buffer, Config};
use iced::widget::{container, row, text, text_input};
use iced::Task;

use self::completion::Completion;
use crate::theme;
use crate::widget::{anchored_overlay, key_press, Element};

mod completion;

pub enum Event {
    InputSent,
}

#[derive(Debug, Clone)]
pub enum Message {
    Input(String),
    Send,
    Tab(bool),
    Up,
    Down,
}

pub fn view<'a>(
    state: &'a State,
    cache: Cache<'a>,
    buffer_focused: bool,
    disabled: bool,
) -> Element<'a, Message> {
    let style = if state.error.is_some() {
        theme::text_input::error
    } else {
        theme::text_input::primary
    };

    let mut text_input = text_input("Send message...", cache.draft)
        .on_submit(Message::Send)
        .id(state.input_id.clone())
        .padding(8)
        .style(style);

    if !disabled {
        text_input = text_input.on_input(Message::Input);
    }

    // Add tab support
    let mut input = key_press(
        key_press(
            text_input,
            key_press::Key::Named(key_press::Named::Tab),
            key_press::Modifiers::SHIFT,
            Message::Tab(true),
        ),
        key_press::Key::Named(key_press::Named::Tab),
        key_press::Modifiers::default(),
        Message::Tab(false),
    );

    // Add up / down support for history cycling
    if buffer_focused {
        input = key_press(
            key_press(
                input,
                key_press::Key::Named(key_press::Named::ArrowUp),
                key_press::Modifiers::default(),
                Message::Up,
            ),
            key_press::Key::Named(key_press::Named::ArrowDown),
            key_press::Modifiers::default(),
            Message::Down,
        );
    }

    let overlay = state
        .error
        .as_deref()
        .map(error)
        .or_else(|| state.completion.view(cache.draft))
        .unwrap_or_else(|| row![].into());

    anchored_overlay(input, overlay, anchored_overlay::Anchor::AboveTop, 4.0)
}

fn error<'a, 'b, Message: 'a>(error: &'b str) -> Element<'a, Message> {
    container(text(error.to_string()).style(theme::text::error))
        .padding(8)
        .style(theme::container::tooltip)
        .into()
}

#[derive(Debug, Clone)]
pub struct State {
    input_id: text_input::Id,
    error: Option<String>,
    completion: Completion,
    selected_history: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            input_id: text_input::Id::unique(),
            error: None,
            completion: Completion::default(),
            selected_history: None,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        buffer: Buffer,
        clients: &mut client::Map,
        history: &mut history::Manager,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Input(input) => {
                // Reset error state
                self.error = None;
                // Reset selected history
                self.selected_history = None;

                let users = buffer
                    .channel()
                    .map(|channel| clients.get_channel_users(buffer.server(), channel))
                    .unwrap_or_default();
                let channels = clients.get_channels(buffer.server());
                let isupport = clients.get_isupport(buffer.server());

                self.completion.process(&input, users, channels, &isupport);

                history.record_draft(Draft {
                    buffer,
                    text: input,
                });

                (Task::none(), None)
            }
            Message::Send => {
                let input = history.input(&buffer).draft;

                // Reset error
                self.error = None;
                // Reset selected history
                self.selected_history = None;

                if let Some(entry) = self.completion.select() {
                    let new_input = entry.complete_input(input);

                    self.on_completion(buffer, history, new_input)
                } else if !input.is_empty() {
                    self.completion.reset();

                    // Parse input
                    let input = match input::parse(
                        buffer.clone(),
                        config.buffer.text_input.auto_format,
                        input,
                    ) {
                        Ok(input) => input,
                        Err(error) => {
                            self.error = Some(error.to_string());
                            return (Task::none(), None);
                        }
                    };

                    if let Some(encoded) = input.encoded() {
                        clients.send(&buffer, encoded);
                    }

                    if let Some(nick) = clients.nickname(buffer.server()) {
                        let mut user = nick.to_owned().into();
                        let mut channel_users = &[][..];

                        // Resolve our attributes if sending this message in a channel
                        if let Buffer::Channel(server, channel) = &buffer {
                            channel_users = clients.get_channel_users(server, channel);

                            if let Some(user_with_attributes) =
                                clients.resolve_user_attributes(server, channel, &user)
                            {
                                user = user_with_attributes.clone();
                            }
                        }

                        history.record_input(input, user, channel_users);
                    }

                    (Task::none(), Some(Event::InputSent))
                } else {
                    (Task::none(), None)
                }
            }
            Message::Tab(reverse) => {
                let input = history.input(&buffer).draft;

                if let Some(entry) = self.completion.tab(reverse) {
                    let new_input = entry.complete_input(input);

                    self.on_completion(buffer, history, new_input)
                } else {
                    (Task::none(), None)
                }
            }
            Message::Up => {
                let cache = history.input(&buffer);

                self.completion.reset();

                if !cache.history.is_empty() {
                    if let Some(index) = self.selected_history.as_mut() {
                        *index = (*index + 1).min(cache.history.len() - 1);
                    } else {
                        self.selected_history = Some(0);
                    }

                    let new_input = cache
                        .history
                        .get(self.selected_history.unwrap())
                        .unwrap()
                        .clone();

                    let users = buffer
                        .channel()
                        .map(|channel| clients.get_channel_users(buffer.server(), channel))
                        .unwrap_or_default();
                    let channels = clients.get_channels(buffer.server());
                    let isupport = clients.get_isupport(buffer.server());

                    self.completion
                        .process(&new_input, users, channels, &isupport);

                    return self.on_completion(buffer, history, new_input);
                }

                (Task::none(), None)
            }
            Message::Down => {
                let cache = history.input(&buffer);

                self.completion.reset();

                if let Some(index) = self.selected_history.as_mut() {
                    let new_input = if *index == 0 {
                        self.selected_history = None;
                        String::new()
                    } else {
                        *index -= 1;
                        let new_input = cache.history.get(*index).unwrap().clone();

                        let users = buffer
                            .channel()
                            .map(|channel| clients.get_channel_users(buffer.server(), channel))
                            .unwrap_or_default();
                        let channels = clients.get_channels(buffer.server());
                        let isupport = clients.get_isupport(buffer.server());

                        self.completion
                            .process(&new_input, users, channels, &isupport);
                        new_input
                    };

                    return self.on_completion(buffer, history, new_input);
                }

                (Task::none(), None)
            }
        }
    }

    fn on_completion(
        &self,
        buffer: Buffer,
        history: &mut history::Manager,
        text: String,
    ) -> (Task<Message>, Option<Event>) {
        history.record_draft(Draft { buffer, text });

        (text_input::move_cursor_to_end(self.input_id.clone()), None)
    }

    pub fn focus(&self) -> Task<Message> {
        text_input::focus(self.input_id.clone())
    }

    pub fn reset(&mut self) {
        self.error = None;
        self.completion = Completion::default();
        self.selected_history = None;
    }

    pub fn insert_user(
        &mut self,
        nick: Nick,
        buffer: Buffer,
        history: &mut history::Manager,
    ) -> Task<Message> {
        let mut text = history.input(&buffer).draft.to_string();

        if text.is_empty() {
            text = format!("{}: ", nick);
        } else if text.ends_with(' ') {
            text = format!("{}{}", text, nick);
        } else {
            text = format!("{} {}", text, nick);
        }

        history.record_draft(Draft { buffer, text });

        text_input::move_cursor_to_end(self.input_id.clone())
    }
}
