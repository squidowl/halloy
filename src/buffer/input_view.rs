use data::buffer::{self, Autocomplete};
use data::dashboard::BufferAction;
use data::input::{self, Cache, Draft};
use data::target::Target;
use data::user::Nick;
use data::{Config, client, command, history};
use iced::Task;
use iced::widget::{column, container, text, text_input};

use self::completion::Completion;
use crate::theme;
use crate::widget::{Element, anchored_overlay, key_press};

mod completion;

pub enum Event {
    InputSent {
        history_task: Task<history::manager::Message>,
    },
    OpenBuffers {
        targets: Vec<(Target, BufferAction)>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Input(String),
    Send,
    Tab(bool),
    Up,
    Down,
    Escape,
}

pub fn view<'a>(
    state: &'a State,
    cache: Cache<'a>,
    buffer_focused: bool,
    disabled: bool,
    config: &Config,
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
                key_press(
                    input,
                    key_press::Key::Named(key_press::Named::ArrowUp),
                    key_press::Modifiers::default(),
                    Message::Up,
                ),
                key_press::Key::Named(key_press::Named::ArrowDown),
                key_press::Modifiers::default(),
                Message::Down,
            ),
            key_press::Key::Named(key_press::Named::Escape),
            key_press::Modifiers::default(),
            Message::Escape,
        );
    }

    let overlay = column![]
        .spacing(4)
        .push_maybe(state.completion.view(cache.draft, config))
        .push_maybe(state.error.as_deref().map(error));

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
        buffer: &buffer::Upstream,
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
                    .map(|channel| {
                        clients.get_channel_users(buffer.server(), channel)
                    })
                    .unwrap_or_default();
                let channels = clients.get_channels(buffer.server());
                let isupport = clients.get_isupport(buffer.server());

                self.completion
                    .process(&input, users, channels, &isupport, config);

                let input =
                    self.completion.complete_emoji(&input).unwrap_or(input);

                if let Err(error) = input::parse(
                    buffer.clone(),
                    config.buffer.text_input.auto_format,
                    &input,
                    &clients.get_isupport(buffer.server()),
                ) {
                    if match error {
                        input::Error::ExceedsByteLimit { .. } => true,
                        input::Error::Command(
                            command::Error::IncorrectArgCount {
                                actual,
                                max,
                                ..
                            },
                        ) => actual > max,
                        input::Error::Command(command::Error::MissingSlash) => {
                            false
                        }
                        input::Error::Command(
                            command::Error::MissingCommand,
                        ) => false,
                        input::Error::Command(
                            command::Error::InvalidModeString,
                        ) => true,
                        input::Error::Command(command::Error::ArgTooLong {
                            ..
                        }) => true,
                        input::Error::Command(
                            command::Error::TooManyTargets { .. },
                        ) => true,
                    } {
                        self.error = Some(error.to_string());
                    }
                }

                history.record_draft(Draft {
                    buffer: buffer.clone(),
                    text: input,
                });

                (Task::none(), None)
            }
            Message::Send => {
                let raw_input = history.input(buffer).draft;

                // Reset error
                self.error = None;
                // Reset selected history
                self.selected_history = None;

                if let Some(entry) = self.completion.select(config) {
                    let chantypes = clients.get_chantypes(buffer.server());
                    let new_input =
                        entry.complete_input(raw_input, chantypes, config);

                    self.on_completion(buffer, history, new_input)
                } else if !raw_input.is_empty() {
                    self.completion.reset();

                    // Parse input
                    let input = match input::parse(
                        buffer.clone(),
                        config.buffer.text_input.auto_format,
                        raw_input,
                        &clients.get_isupport(buffer.server()),
                    ) {
                        Ok(input::Parsed::Internal(command)) => {
                            history.record_input_history(
                                buffer,
                                raw_input.to_owned(),
                            );

                            match command {
                                command::Internal::OpenBuffers(targets) => {
                                    let chantypes =
                                        clients.get_chantypes(buffer.server());
                                    let statusmsg =
                                        clients.get_statusmsg(buffer.server());
                                    let casemapping = clients
                                        .get_casemapping(buffer.server());

                                    return (
                                        Task::none(),
                                        Some(Event::OpenBuffers {
                                            targets: targets
                                                .split(",")
                                                .map(|target| {
                                                    Target::parse(
                                                        target,
                                                        chantypes,
                                                        statusmsg,
                                                        casemapping,
                                                    )
                                                })
                                                .map(|target| match target {
                                                    Target::Channel(_) => (
                                                        target,
                                                        config
                                                            .actions
                                                            .buffer
                                                            .message_channel,
                                                    ),
                                                    Target::Query(_) => (
                                                        target,
                                                        config
                                                            .actions
                                                            .buffer
                                                            .message_user,
                                                    ),
                                                })
                                                .collect(),
                                        }),
                                    );
                                }
                            }
                        }
                        Ok(input::Parsed::Input(input)) => input,
                        Err(error) => {
                            self.error = Some(error.to_string());
                            return (Task::none(), None);
                        }
                    };

                    history.record_input_history(buffer, raw_input.to_owned());

                    if let Some(encoded) = input.encoded() {
                        clients.send(buffer, encoded);
                    }

                    let mut history_task = Task::none();

                    if let Some(nick) = clients.nickname(buffer.server()) {
                        let mut user = nick.to_owned().into();
                        let mut channel_users = &[][..];
                        let chantypes = clients.get_chantypes(buffer.server());
                        let statusmsg = clients.get_statusmsg(buffer.server());
                        let casemapping =
                            clients.get_casemapping(buffer.server());

                        // Resolve our attributes if sending this message in a channel
                        if let buffer::Upstream::Channel(server, channel) =
                            &buffer
                        {
                            channel_users =
                                clients.get_channel_users(server, channel);

                            if let Some(user_with_attributes) = clients
                                .resolve_user_attributes(server, channel, &user)
                            {
                                user = user_with_attributes.clone();
                            }
                        }

                        history_task = Task::batch(
                            history
                                .record_input_message(
                                    input,
                                    user,
                                    channel_users,
                                    chantypes,
                                    statusmsg,
                                    casemapping,
                                    config,
                                )
                                .into_iter()
                                .map(Task::future),
                        );
                    }

                    (Task::none(), Some(Event::InputSent { history_task }))
                } else {
                    (Task::none(), None)
                }
            }
            Message::Tab(reverse) => {
                let input = history.input(buffer).draft;

                if let Some(entry) = self.completion.tab(reverse) {
                    let chantypes = clients.get_chantypes(buffer.server());
                    let new_input =
                        entry.complete_input(input, chantypes, config);

                    self.on_completion(buffer, history, new_input)
                } else {
                    (Task::none(), None)
                }
            }
            Message::Up => {
                if self.completion.arrow(completion::Arrow::Up) {
                    return (Task::none(), None);
                }

                let cache = history.input(buffer);

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
                        .map(|channel| {
                            clients.get_channel_users(buffer.server(), channel)
                        })
                        .unwrap_or_default();
                    let channels = clients.get_channels(buffer.server());
                    let isupport = clients.get_isupport(buffer.server());

                    self.completion.process(
                        &new_input, users, channels, &isupport, config,
                    );

                    return self.on_completion(buffer, history, new_input);
                }

                (Task::none(), None)
            }
            Message::Down => {
                if self.completion.arrow(completion::Arrow::Down) {
                    return (Task::none(), None);
                }

                let cache = history.input(buffer);

                self.completion.reset();

                if let Some(index) = self.selected_history.as_mut() {
                    let new_input = if *index == 0 {
                        self.selected_history = None;
                        String::new()
                    } else {
                        *index -= 1;
                        let new_input =
                            cache.history.get(*index).unwrap().clone();

                        let users = buffer
                            .channel()
                            .map(|channel| {
                                clients
                                    .get_channel_users(buffer.server(), channel)
                            })
                            .unwrap_or_default();
                        let channels = clients.get_channels(buffer.server());
                        let isupport = clients.get_isupport(buffer.server());

                        self.completion.process(
                            &new_input, users, channels, &isupport, config,
                        );
                        new_input
                    };

                    return self.on_completion(buffer, history, new_input);
                }

                (Task::none(), None)
            }
            // Capture escape so that closing context menu or commands/emojis picker
            // does not defocus input
            Message::Escape => (Task::none(), None),
        }
    }

    fn on_completion(
        &self,
        buffer: &buffer::Upstream,
        history: &mut history::Manager,
        text: String,
    ) -> (Task<Message>, Option<Event>) {
        history.record_draft(Draft {
            buffer: buffer.clone(),
            text,
        });

        (text_input::move_cursor_to_end(self.input_id.clone()), None)
    }

    pub fn focus(&self) -> Task<Message> {
        let input_id = self.input_id.clone();

        text_input::is_focused(input_id.clone()).then(move |is_focused| {
            if is_focused {
                Task::none()
            } else {
                text_input::focus(input_id.clone())
            }
        })
    }

    pub fn reset(&mut self) {
        self.error = None;
        self.completion = Completion::default();
        self.selected_history = None;
    }

    pub fn insert_user(
        &mut self,
        nick: Nick,
        buffer: buffer::Upstream,
        history: &mut history::Manager,
        autocomplete: &Autocomplete,
    ) -> Task<Message> {
        let mut text = history.input(&buffer).draft.to_string();

        let suffix = if text.is_empty() {
            text = format!("{nick}");

            &autocomplete.completion_suffixes[0]
        } else {
            if text.ends_with(' ') {
                text = format!("{text}{nick}");
            } else {
                text = format!("{text} {nick}");
            }

            &autocomplete.completion_suffixes[1]
        };
        text.push_str(suffix);

        history.record_draft(Draft { buffer, text });

        text_input::move_cursor_to_end(self.input_id.clone())
    }

    pub fn close_picker(&mut self) -> bool {
        self.completion.close_picker()
    }
}
