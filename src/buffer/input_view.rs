use std::time::Duration;

use data::buffer::{self, Autocomplete, Upstream};
use data::dashboard::BufferAction;
use data::history::{self, ReadMarker};
use data::input::{self, Cache, RawInput};
use data::message::server_time;

// there is probably a better way to do this
#[cfg(feature = "hexchat-compat")]
use data::Command;
#[cfg(feature = "hexchat-compat")]
use data::python::{self, HalloyAction, RustpythonExec, run_hook};
#[cfg(feature = "hexchat-compat")]
use once_cell::sync::Lazy;
#[cfg(feature = "hexchat-compat")]
use rustpython_vm::{Interpreter, scope::Scope};
#[cfg(feature = "hexchat-compat")]
use std::cell::RefCell;
#[cfg(feature = "hexchat-compat")]
use std::collections::HashMap;
#[cfg(feature = "hexchat-compat")]
use std::fs;
#[cfg(feature = "hexchat-compat")]
use std::path::PathBuf;
#[cfg(feature = "hexchat-compat")]
use std::rc::Rc;

#[cfg(not(feature = "hexchat-compat"))]
fn run_hook(
    _: Option<&buffer::Upstream>,
    _: String,
    _: Vec<String>,
    _: bool,
    _: bool,
) {
}

use data::target::Target;
use data::user::Nick;
use data::{Config, client, command};
use iced::Task;
use iced::widget::{column, container, text, text_input};
use irc::proto;
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

#[cfg(feature = "hexchat-compat")]
thread_local! {
    static PY_SCOPES: Lazy<RefCell<HashMap<String, Option<Scope>>>> = Lazy::new(|| RefCell::new(HashMap::new()));
}
#[cfg(feature = "hexchat-compat")]
thread_local! {
    static PY_INTERPRS: Lazy<RefCell<HashMap<String, Option<Rc<Interpreter>>>>> = Lazy::new(|| RefCell::new(HashMap::new()));
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

    let mut text_input = text_input("Send message...", cache.text)
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
        .push_maybe(state.completion.view(cache.text, config))
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

    #[cfg(feature = "hexchat-compat")]
    pub fn python(code: String) {
        let mut scope: Option<Scope> = None;
        let mut inter: Option<Rc<Interpreter>> = None;
        PY_SCOPES.with(|scopes| {
            let scopes = scopes.borrow();
            match scopes.clone().get("console") {
                Some(console_scope) => {
                    scope = console_scope.clone();
                }

                None => {}
            }
        });
        PY_INTERPRS.with(|inters| {
            let inters = inters.borrow();
            if let Some(console_inter) = inters.get("console") {
                inter = Some(console_inter.clone().unwrap());
            }
        });
        let rpexec = RustpythonExec {
            cmd: code.to_string().clone(),
            scope: scope.clone(),
            interp: inter,
            clear_actions: true,
        };
        let result = python::exec(rpexec);
        PY_SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            scopes.insert("console".to_owned(), Some(result.scope.clone()))
        });
        PY_INTERPRS.with(|inters| {
            let mut inters = inters.borrow_mut();
            inters.insert("console".to_owned(), Some(result.interp));
        });
        let txt: String;

        if let Some(err) = result.error {
            txt = err
        } else {
            txt = result.out.clone()
        };

        for action_ in result.actions.clone() {
            if let Some(action) = action_ {
                match action {
                    HalloyAction::Print(string) => {
                        if string.clone().replace("\n", "") != "" {
                            python::print_to_log(string);
                        }
                    }

                    HalloyAction::Hook(hooks) => {
                        for hook in hooks {
                            data::python::append_to_hooks(hook);
                        }
                    }

                    HalloyAction::Command(cmd) => {
                        match command::parse(&cmd, None, &HashMap::new()) {
                            Ok(parsed_cmd) => {
                                python::push_command(parsed_cmd.clone());
                            }

                            Err(_) => {
                                log::debug!(
                                    "py: user passed invalid command in python!"
                                );
                            }
                        }
                    }
                }
            }
        }

        if txt.clone().replace("\n", "") != "" {
            python::print_to_log(txt.clone());
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
        let current_channel = buffer.channel();
        #[cfg(feature = "hexchat-compat")]
        python::print_queue(buffer, history, config);

        #[cfg(feature = "hexchat-compat")]
        let actions = python::get_actions();
        #[cfg(feature = "hexchat-compat")]
        for action_ in actions.clone() {
            if let Some(action) = action_ {
                match action {
                    HalloyAction::Print(string) => {
                        if string.clone().replace("\n", "") != "" {
                            python::print_to_log(string);
                        }
                    }

                    HalloyAction::Hook(hooks) => {
                        for hook in hooks {
                            data::python::append_to_hooks(hook);
                        }
                    }

                    HalloyAction::Command(cmd) => {
                        match command::parse(&cmd, None, &HashMap::new()) {
                            Ok(parsed_cmd) => {
                                python::push_command(parsed_cmd.clone());
                            }

                            Err(_) => {
                                log::debug!(
                                    "py: user passed invalid command in python!"
                                );
                            }
                        }
                    }
                }
            }
        }

        #[cfg(feature = "hexchat-compat")]
        for cmd in python::list_commands() {
            if let Command::Irc(irc_cmd) = cmd {
                log::debug!("py: running IRC command from command queue!");

                let command;

                match irc_cmd.clone() {
                    command::Irc::Unknown(key, value) => {
                        if key.to_lowercase() == "me".to_lowercase().to_owned()
                        {
                            command = data::Input::command(
                                buffer.clone(),
                                command::Irc::Me(
                                    buffer
                                        .channel()
                                        .unwrap()
                                        .to_owned()
                                        .to_string(),
                                    value.join(" "),
                                ),
                            )
                            .encoded()
                            .unwrap();
                        } else {
                            command = data::Input::command(
                                buffer.clone(),
                                irc_cmd.clone(),
                            )
                            .encoded()
                            .unwrap();
                        }
                    }

                    _ => {
                        command = data::Input::command(
                            buffer.clone(),
                            irc_cmd.clone(),
                        )
                        .encoded()
                        .unwrap();
                    }
                }
                clients
                    .client_mut(buffer.server())
                    .unwrap()
                    .send(buffer, command);
            }

            // Internal commands are not handled (yet), but no HexChat plugin seems to use them...
        }

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
                    current_channel,
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
                        input::Error::Command(
                            command::Error::NotPositiveInteger,
                        ) => true,
                    } {
                        self.error = Some(error.to_string());
                    }
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
                        &clients.get_isupport(buffer.server()),
                    ) {
                        Ok(input::Parsed::Internal(command)) => {
                            history.record_input_history(
                                buffer,
                                raw_input.to_owned(),
                            );

                            match command {
                                #[cfg(feature = "hexchat-compat")]
                                #[allow(unused_variables, unused_assignments)]
                                command::Internal::Py(cmd, args_optional) => {
                                    let mut event: Option<Event> = None;
                                    match cmd.as_str() {
                                        "console" | "CONSOLE" => {
                                            let chantypes = clients
                                                .get_chantypes(buffer.server());
                                            let statusmsg = clients
                                                .get_statusmsg(buffer.server());
                                            let casemapping = clients
                                                .get_casemapping(
                                                    buffer.server(),
                                                );

                                            let target = Target::parse(
                                                "python-log",
                                                chantypes,
                                                statusmsg,
                                                casemapping,
                                            );

                                            let buffer_action = config
                                                .actions
                                                .buffer
                                                .message_channel;

                                            event = Some(Event::OpenBuffers {
                                                targets: vec![(
                                                    target,
                                                    buffer_action,
                                                )],
                                            })
                                        }

                                        "load" | "LOAD" => {
                                            if let Some(args) = args_optional {
                                                if args.len() >= 1 {
                                                    let path = PathBuf::from(
                                                        args[0].clone(),
                                                    );
                                                    if path.exists()
                                                        && (path.is_file()
                                                            || path
                                                                .is_symlink())
                                                    {
                                                        let code =
                                                            fs::read_to_string(
                                                                path,
                                                            )
                                                            .unwrap();
                                                        Self::python(code);
                                                    } else {
                                                        self.error = Some("Script does not exist or is a directory!".to_owned());
                                                    }
                                                } else {
                                                    self.error = Some(
                                                        "No path specified..."
                                                            .to_owned(),
                                                    );
                                                }
                                            } else {
                                                self.error = Some("No path specified, or path is invalid...".to_owned());
                                            }
                                        }

                                        "help" | "HELP" => {
                                            self.error = Some(
                                                "Available commands:
                                                - /py load {filename} - load a HexChat or XChat extension at {filename}
                                                - /py console - open a interactive console buffer".to_owned()
                                            )
                                        }

                                        cmd => {
                                            if !python::command_hooked(cmd.to_owned().clone()) {
                                                self.error = Some(
                                                    "Invalid command specified. Check /py help."
                                                        .to_owned(),
                                                )
                                            };

                                            let mut words_: Vec<String> = Vec::new();
                                            if let Some(args) = args_optional {
                                                words_ = args.clone()
                                            }

                                            let mut words: Vec<String> = vec![cmd.to_owned().clone()];
                                            for word in words_ {
                                                words.push(word.to_owned());
                                            }

                                            python::run_hook(None, cmd.to_owned(), words, false, true);
                                            // ensure the result shows up
                                            python::print_queue(buffer, history, config);
                                        }
                                    }
                                    return (Task::none(), event);
                                }
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
                                // Ignore any delay command sent from input.
                                command::Internal::Delay(_) => {
                                    return (Task::none(), None);
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
                    #[cfg(feature = "hexchat-compat")]
                    let mut send_user_message: bool = true;
                    #[cfg(not(feature = "hexchat-compat"))]
                    let send_user_message = true;

                    if let Some(encoded) = input.encoded() {
                        let sent_time = server_time(&encoded);

                        run_hook(
                            Some(buffer),
                            encoded.command.clone().command(),
                            encoded
                                .command
                                .clone()
                                .parameters()
                                .drain(1..)
                                .collect(),
                            true,
                            false,
                        );

                        #[allow(unused_variables)]
                        if let proto::Command::PRIVMSG(target, msg) =
                            &encoded.command.clone()
                        {
                            #[cfg(feature = "hexchat-compat")]
                            if *target == "python-log".to_owned() {
                                // let the user message show up first
                                if let Some(nick) =
                                    clients.nickname(buffer.server())
                                {
                                    let mut user = nick.to_owned().into();
                                    let mut channel_users = &[][..];

                                    let chantypes =
                                        clients.get_chantypes(buffer.server());
                                    let statusmsg =
                                        clients.get_statusmsg(buffer.server());
                                    let casemapping = clients
                                        .get_casemapping(buffer.server());

                                    // Resolve our attributes if sending this message in a channel
                                    if let buffer::Upstream::Channel(
                                        server,
                                        channel,
                                    ) = &buffer
                                    {
                                        channel_users = clients
                                            .get_channel_users(server, channel);

                                        if let Some(user_with_attributes) =
                                            clients.resolve_user_attributes(
                                                server, channel, &user,
                                            )
                                        {
                                            user = user_with_attributes.clone();
                                        }
                                    }
                                    history.record_input_message(
                                        input.clone(),
                                        user,
                                        channel_users,
                                        chantypes,
                                        statusmsg,
                                        casemapping,
                                        config,
                                    );
                                }

                                send_user_message = false;
                                Self::python(msg.clone());
                                // ensure the result shows up
                                python::print_queue(buffer, history, config);
                            };
                        }

                        clients.send(buffer, encoded);

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
                                    );
                                }
                            }
                        }
                    }

                    let mut history_task = Task::none();

                    if !send_user_message {
                        return (Task::none(), None);
                    }

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
                        current_channel,
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
                            current_channel,
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
