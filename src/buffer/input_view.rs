use std::time::Duration;

use data::buffer::{self, Autocomplete, Upstream};
use data::dashboard::BufferAction;
use data::history::{self, ReadMarker};
use data::input::{self, Cache, RawInput};
use data::message::server_time;
use data::rate_limit::TokenPriority;
use data::target::Target;
use data::user::Nick;
use data::{Config, User, client, command};
use iced::widget::{column, container, row, text, text_input, vertical_rule};
use iced::{Alignment, Task, padding};
use tokio::time;

use self::completion::Completion;
use crate::widget::{Element, anchored_overlay, key_press};
use crate::{Theme, font, theme};

mod completion;

pub enum Event {
    InputSent {
        history_task: Task<history::manager::Message>,
    },
    OpenBuffers {
        targets: Vec<(Target, BufferAction)>,
    },
    LeaveBuffers {
        targets: Vec<Target>,
        reason: Option<String>,
    },
    Cleared {
        history_task: Task<history::manager::Message>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    SysInfoReceived(iced::system::Information),
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
    our_user: Option<&User>,
    disabled: bool,
    config: &Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let style = if state.error.is_some() {
        theme::text_input::error
    } else {
        theme::text_input::primary
    };

    let mut text_input = text_input("Send message...", cache.text)
        .on_submit(Message::Send)
        .id(state.input_id.clone())
        .padding([0, 4])
        .style(style);

    if !disabled {
        text_input = text_input.on_input(Message::Input);
    }

    // Add tab support
    let input = key_press(
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

    let our_user_style = {
        let is_user_away = config
            .buffer
            .nickname
            .away
            .is_away(our_user.is_none_or(User::is_away));

        let seed = match config.buffer.nickname.color {
            data::buffer::Color::Solid => None,
            data::buffer::Color::Unique => {
                our_user.map(|user| Some(user.seed()))
            }
        }
        .flatten();

        theme::text::nickname(theme, seed, is_user_away, false)
    };

    let maybe_our_user =
        config.buffer.text_input.show_own_nickname.then(move || {
            our_user.map(|user| {
                container(
                    text(user.display(true, None))
                        .style(move |_| our_user_style)
                        .font_maybe(
                            theme::font_style::nickname(theme, false)
                                .map(font::get),
                        ),
                )
                .padding(padding::right(4).left(2))
            })
        });

    let maybe_vertical_rule =
        maybe_our_user.is_some().then(move || vertical_rule(1.0));

    let mut content = column![
        container(
            row![maybe_our_user, maybe_vertical_rule, input]
                .spacing(4)
                .height(22)
                .align_y(Alignment::Center)
        )
        .padding([8, 14])
        .style(theme::container::buffer_text_input),
    ]
    .spacing(4)
    .into();

    // Add up / down support for history cycling
    if buffer_focused {
        content = key_press(
            key_press(
                key_press(
                    content,
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

    let overlay = column![
        state.completion.view(cache.text, config, theme),
        state
            .error
            .as_deref()
            .map(|error_str| error(error_str, theme)),
    ]
    .padding([0, 8])
    .spacing(4);

    anchored_overlay(content, overlay, anchored_overlay::Anchor::AboveTop, 4.0)
}

fn error<'a, 'b, Message: 'a>(
    error: &'b str,
    theme: &'a Theme,
) -> Element<'a, Message> {
    container(
        text(error.to_string())
            .style(theme::text::error)
            .font_maybe(theme::font_style::error(theme).map(font::get)),
    )
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
        let current_target = buffer.target();

        match message {
            Message::SysInfoReceived(info) => {
                let sysinfo_config = &config.buffer.commands.sysinfo;

                let sysinfo_parts = [
                    // OS
                    sysinfo_config.os.then(|| {
                        info.system_version.as_deref().map_or_else(
                            || "OS: Unknown".to_string(),
                            |version| {
                                if let Some(kernel) = &info.system_kernel {
                                    format!("OS: {version} ({kernel})")
                                } else {
                                    format!("OS: {version}")
                                }
                            },
                        )
                    }),
                    // CPU
                    sysinfo_config
                        .cpu
                        .then(|| format!("CPU: {}", info.cpu_brand.trim())),
                    // Memory
                    sysinfo_config.memory.then(|| {
                        let total_gb = (info.memory_total as f64
                            / (1024.0 * 1024.0 * 1024.0))
                            .ceil()
                            as u64;
                        format!("MEM: {total_gb} GB")
                    }),
                    // GPU
                    sysinfo_config.gpu.then(|| {
                        format!(
                            "GPU: {} ({})",
                            info.graphics_adapter.trim(),
                            info.graphics_backend.trim()
                        )
                    }),
                    // Uptime
                    sysinfo_config
                        .uptime
                        .then(|| {
                            uptime_lib::get().ok().map(|uptime| {
                                let mut formatter = timeago::Formatter::new();
                                formatter.num_items(4);
                                format!("UP: {}", formatter.convert(uptime))
                            })
                        })
                        .flatten(),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

                // If no sysinfo is enabled, don't send anything
                if sysinfo_parts.is_empty() {
                    return (Task::none(), None);
                }

                let message = sysinfo_parts.join(" ");

                history.record_input_history(buffer, message.clone());

                if let Ok(data::input::Parsed::Input(input)) = input::parse(
                    buffer.clone(),
                    config.buffer.text_input.auto_format,
                    message.as_str(),
                    clients.nickname(buffer.server()),
                    &clients.get_isupport(buffer.server()),
                ) && let Some(encoded) = input.encoded()
                {
                    clients.send(buffer, encoded, TokenPriority::User);
                }

                (Task::none(), None)
            }
            Message::Input(input) => {
                // Reset error state
                self.error = None;
                // Reset selected history
                self.selected_history = None;

                let users = buffer.channel().and_then(|channel| {
                    clients.get_channel_users(buffer.server(), channel)
                });
                // TODO(pounce) eliminate clones
                let channels = clients
                    .get_channels(buffer.server())
                    .cloned()
                    .collect::<Vec<_>>();
                let supports_detach =
                    clients.get_server_supports_detach(buffer.server());
                let isupport = clients.get_isupport(buffer.server());

                self.completion.process(
                    &input,
                    clients.nickname(buffer.server()),
                    users,
                    &history.get_last_seen(buffer),
                    &channels,
                    current_target.as_ref(),
                    supports_detach,
                    &isupport,
                    config,
                );

                let input =
                    self.completion.complete_emoji(&input).unwrap_or(input);

                if let Err(error) = input::parse(
                    buffer.clone(),
                    config.buffer.text_input.auto_format,
                    &input,
                    clients.nickname(buffer.server()),
                    &clients.get_isupport(buffer.server()),
                ) && match error {
                    input::Error::ExceedsByteLimit { .. } => true,
                    input::Error::Command(
                        command::Error::IncorrectArgCount {
                            actual, max, ..
                        },
                    ) => actual > max,
                    input::Error::Command(command::Error::MissingSlash) => {
                        false
                    }
                    input::Error::Command(command::Error::MissingCommand) => {
                        false
                    }
                    input::Error::Command(command::Error::NoModeString) => {
                        false
                    }
                    input::Error::Command(
                        command::Error::InvalidModeString,
                    ) => true,
                    input::Error::Command(command::Error::ArgTooLong {
                        ..
                    }) => true,
                    input::Error::Command(command::Error::TooManyTargets {
                        ..
                    }) => true,
                    input::Error::Command(
                        command::Error::NotPositiveInteger,
                    ) => true,
                    input::Error::Command(
                        command::Error::InvalidChannelName { .. },
                    ) => true,
                } {
                    self.error = Some(error.to_string());
                }

                history.record_text(RawInput {
                    buffer: buffer.clone(),
                    text: input.clone(),
                });

                history.record_draft(RawInput {
                    buffer: buffer.clone(),
                    text: input,
                });

                (Task::none(), None)
            }
            Message::Send => {
                let raw_input = history.input(buffer).text;

                // Reset error
                self.error = None;
                // Reset selected history
                self.selected_history = None;

                if let Some(entry) = self.completion.select(config) {
                    let chantypes = clients.get_chantypes(buffer.server());
                    let new_input =
                        entry.complete_input(raw_input, chantypes, config);

                    self.on_completion(buffer, history, new_input, true)
                } else if !raw_input.is_empty() {
                    self.completion.reset();

                    // Parse input
                    let input = match input::parse(
                        buffer.clone(),
                        config.buffer.text_input.auto_format,
                        raw_input,
                        clients.nickname(buffer.server()),
                        &clients.get_isupport(buffer.server()),
                    ) {
                        Ok(input::Parsed::Internal(command)) => {
                            history.record_input_history(
                                buffer,
                                raw_input.to_owned(),
                            );

                            match command {
                                command::Internal::OpenBuffers(targets) => {
                                    return (
                                        Task::none(),
                                        Some(Event::OpenBuffers {
                                            targets: targets
                                                .into_iter()
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
                                command::Internal::LeaveBuffers(
                                    targets,
                                    reason,
                                ) => {
                                    return (
                                        Task::none(),
                                        Some(Event::LeaveBuffers {
                                            targets,
                                            reason,
                                        }),
                                    );
                                }
                                command::Internal::Detach(channels) => {
                                    return (
                                        Task::none(),
                                        Some(Event::LeaveBuffers {
                                            targets: channels
                                                .into_iter()
                                                .map(Target::Channel)
                                                .collect(),
                                            reason: Some("detach".to_string()),
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

                                    // Part channel. Might not exist if we execute on a query/server.
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
                                        clients.send(
                                            buffer,
                                            part_command,
                                            TokenPriority::User,
                                        );
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
                                command::Internal::Delay(_) => {
                                    return (Task::none(), None);
                                }
                                command::Internal::ClearBuffer => {
                                    let kind = history::Kind::from_input_buffer(
                                        buffer.clone(),
                                    );

                                    let event = history
                                        .clear_messages(kind)
                                        .map(|history_task| Event::Cleared {
                                            history_task: Task::future(
                                                history_task,
                                            ),
                                        });

                                    return (Task::none(), event);
                                }
                                command::Internal::SysInfo => {
                                    return (
                                        iced::system::information()
                                            .map(Message::SysInfoReceived),
                                        None,
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
                        let sent_time = server_time(&encoded);

                        clients.send(buffer, encoded, TokenPriority::User);

                        if config.buffer.mark_as_read.on_message_sent {
                            let chantypes =
                                clients.get_chantypes(buffer.server());
                            let statusmsg =
                                clients.get_statusmsg(buffer.server());
                            let casemapping =
                                clients.get_casemapping(buffer.server());

                            if let Some(targets) =
                                input.targets(chantypes, statusmsg, casemapping)
                            {
                                for target in targets {
                                    clients.send_markread(
                                        buffer.server(),
                                        target,
                                        ReadMarker::from_date_time(sent_time),
                                        TokenPriority::High,
                                    );
                                }
                            }
                        }
                    }

                    let mut history_task = Task::none();

                    if let Some(nick) = clients.nickname(buffer.server()) {
                        let mut user = nick.to_owned().into();
                        let mut channel_users = None;

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
                let input = history.input(buffer).text;

                if let Some(entry) = self.completion.tab(reverse) {
                    let chantypes = clients.get_chantypes(buffer.server());
                    let new_input =
                        entry.complete_input(input, chantypes, config);

                    self.on_completion(buffer, history, new_input, true)
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

                    let users = buffer.channel().and_then(|channel| {
                        clients.get_channel_users(buffer.server(), channel)
                    });
                    let channels = clients
                        .get_channels(buffer.server())
                        .cloned()
                        .collect::<Vec<_>>();
                    let supports_detach =
                        clients.get_server_supports_detach(buffer.server());
                    let isupport = clients.get_isupport(buffer.server());

                    self.completion.process(
                        &new_input,
                        clients.nickname(buffer.server()),
                        users,
                        &history.get_last_seen(buffer),
                        &channels,
                        current_target.as_ref(),
                        supports_detach,
                        &isupport,
                        config,
                    );

                    return self
                        .on_completion(buffer, history, new_input, false);
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
                        cache.draft.to_string()
                    } else {
                        *index -= 1;
                        let new_input =
                            cache.history.get(*index).unwrap().clone();

                        let users = buffer.channel().and_then(|channel| {
                            clients.get_channel_users(buffer.server(), channel)
                        });
                        let channels = clients
                            .get_channels(buffer.server())
                            .cloned()
                            .collect::<Vec<_>>();
                        let supports_detach =
                            clients.get_server_supports_detach(buffer.server());
                        let isupport = clients.get_isupport(buffer.server());

                        self.completion.process(
                            &new_input,
                            clients.nickname(buffer.server()),
                            users,
                            &history.get_last_seen(buffer),
                            &channels,
                            current_target.as_ref(),
                            supports_detach,
                            &isupport,
                            config,
                        );
                        new_input
                    };

                    return self
                        .on_completion(buffer, history, new_input, false);
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
                    clients.send(&buffer, input, TokenPriority::User);
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
        record_draft: bool,
    ) -> (Task<Message>, Option<Event>) {
        history.record_text(RawInput {
            buffer: buffer.clone(),
            text: text.clone(),
        });

        if record_draft {
            history.record_draft(RawInput {
                buffer: buffer.clone(),
                text,
            });
        }

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
        let mut text = history.input(&buffer).text.to_string();

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

        history.record_text(RawInput {
            buffer: buffer.clone(),
            text: text.clone(),
        });

        history.record_draft(RawInput { buffer, text });

        text_input::move_cursor_to_end(self.input_id.clone())
    }

    pub fn close_picker(&mut self) -> bool {
        self.completion.close_picker()
    }
}
