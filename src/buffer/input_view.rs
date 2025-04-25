use data::buffer::{self, Autocomplete};
use std::time::Duration;

use data::buffer::Upstream;
use data::dashboard::BufferAction;
use data::input::{self, Cache, Draft};
use data::target::Target;
use data::user::Nick;
use data::{Config, client, command, history};
use iced::Task;
use iced::widget::{column, container, text, text_input};
use tokio::time;

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
    SendCommand {
        buffer: Upstream,
        command: command::Irc,
    },
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

                self.completion.process(
                    &input,
                    users,
                    &history.get_last_seen(buffer),
                    channels,
                    &isupport,
                    config,
                );

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
                                command::Internal::Hop(first, rest) => {
                                    let has_channel_argument = first
                                        .as_ref()
                                        .is_some_and(|s| s.starts_with('#'));

                                    // Channel to join, either from first argument or buffer channel
                                    let target_channel = if has_channel_argument
                                    {
                                        // Use first argument as channel.
                                        first.clone()
                                    } else {
                                        // If first argument isn't a channel, we use buffer channel
                                        buffer.channel().map(|chan| {
                                            chan.as_str().to_string()
                                        })
                                    };

                                    // If we don't have a target channel for some reason we return
                                    let Some(target_channel) = target_channel
                                    else {
                                        return (Task::none(), None);
                                    };

                                    let message = if has_channel_argument {
                                        // If first argument is a channel, we use second argument as message
                                        rest
                                    } else {
                                        // Otherwise we use both arguments
                                        match (
                                            first.as_deref(),
                                            rest.as_deref(),
                                        ) {
                                            (Some(a), Some(b)) => {
                                                Some(format!("{a} {b}"))
                                            }
                                            (Some(a), None) => {
                                                Some(a.to_string())
                                            }
                                            (None, Some(b)) => {
                                                Some(b.to_string())
                                            }
                                            (None, None) => None,
                                        }
                                    };

                                    // Part channel. Might not exsist if we execute on a query/server.
                                    let part_command =
                                        buffer.channel().and_then(|channel| {
                                            data::Input::command(
                                                buffer.clone(),
                                                command::Irc::Part(
                                                    channel
                                                        .as_str()
                                                        .to_string(),
                                                    message,
                                                ),
                                            )
                                            .encoded()
                                        });

                                    // Send part command.
                                    if let Some(part_command) = part_command {
                                        clients.send(buffer, part_command);
                                    }

                                    // Create a delay task that will execute the join after waiting
                                    let buffer_clone = buffer.clone();
                                    let target_channel_clone =
                                        target_channel.clone();

                                    let delayed_join_task = Task::perform(
                                        time::sleep(Duration::from_millis(100)),
                                        move |()| Message::SendCommand {
                                            buffer: buffer_clone,
                                            command: command::Irc::Join(
                                                target_channel_clone,
                                                None,
                                            ),
                                        },
                                    );

                                    let chantypes =
                                        clients.get_chantypes(buffer.server());
                                    let statusmsg =
                                        clients.get_statusmsg(buffer.server());
                                    let casemapping = clients
                                        .get_casemapping(buffer.server());

                                    let target = Target::parse(
                                        target_channel.as_str(),
                                        chantypes,
                                        statusmsg,
                                        casemapping,
                                    );

                                    let event =
                                        has_channel_argument.then_some({
                                            let buffer_action = match buffer {
                                                // If it's a channel, we want to replace it when hopping to a new channel.
                                                Upstream::Channel(..) => {
                                                    BufferAction::ReplacePane
                                                }
                                                // If it's a server or query, we want to follow config for actions.
                                                Upstream::Server(..)
                                                | Upstream::Query(..) => {
                                                    config
                                                        .actions
                                                        .buffer
                                                        .message_channel
                                                }
                                            };

                                            Event::OpenBuffers {
                                                targets: vec![(
                                                    target,
                                                    buffer_action,
                                                )],
                                            }
                                        });

                                    return (delayed_join_task, event);
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
                        &new_input,
                        users,
                        &history.get_last_seen(buffer),
                        channels,
                        &isupport,
                        config,
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
                            &new_input,
                            users,
                            &history.get_last_seen(buffer),
                            channels,
                            &isupport,
                            config,
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
            Message::SendCommand { buffer, command } => {
                let input =
                    data::Input::command(buffer.clone(), command).encoded();

                // Send command.
                if let Some(input) = input {
                    clients.send(&buffer, input);
                }

                (Task::none(), None)
            }
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
