use std::borrow::Cow;
use std::convert;
use std::time::Duration;

use data::buffer::{self, Upstream};
use data::config::buffer::text_input::{Autocomplete, KeyBindings};
use data::dashboard::BufferAction;
use data::history::filter::FilterChain;
use data::history::{self, ReadMarker};
use data::input::{self, RawInput};
use data::message::server_time;
use data::rate_limit::TokenPriority;
use data::server::Server;
use data::target::Target;
use data::user::Nick;
use data::{Config, User, client, command, shortcut};
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, mouse};
use iced::widget::text::{Shaping, Wrapping};
use iced::widget::{
    self, button, column, container, operation, row, rule, text_editor,
};
use iced::{Alignment, Length, Task, clipboard, event, keyboard, padding};
use tokio::time;

use self::completion::Completion;
use crate::widget::key_press::is_numpad;
use crate::widget::{
    Element, Renderer, Text, anchored_overlay, context_menu, decorate, text,
};
use crate::window::Window;
use crate::{Theme, font, theme, window};

mod completion;

pub enum Event {
    InputSent {
        history_task: Task<history::manager::Message>,
        open_buffers: Vec<(Target, BufferAction)>,
    },
    OpenBuffers {
        server: Server,
        targets: Vec<(Target, BufferAction)>,
    },
    OpenInternalBuffer(buffer::Internal),
    OpenServer(String),
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
    Action(text_editor::Action),
    CloseContextMenu(window::Id, bool),
    SysInfoReceived(iced::system::Information),
    Send,
    DeleteWordForward(bool),
    DeleteWordBackward(bool),
    DeleteToEnd(bool),
    DeleteToStart(bool),
    Tab(bool),
    Up,
    Down,
    Escape,
    SendCommand {
        buffer: Upstream,
        command: command::Irc,
    },
    Paste,
    SelectAll,
    CopyAll,
    Copy,
    Cut,
}

#[derive(Debug, Clone, Copy)]
pub enum Actions {
    Cut,
    Copy,
    CopyAll,
    Paste,
    SelectAll,
}

impl Actions {
    fn list() -> Vec<Self> {
        vec![
            Self::Cut,
            Self::Copy,
            Self::CopyAll,
            Self::Paste,
            Self::SelectAll,
        ]
    }
}

fn emacs_key_binding(
    key_press: text_editor::KeyPress,
) -> Option<text_editor::Binding<Message>> {
    match key_press.key.as_ref() {
        iced::keyboard::Key::Character("e")
            if key_press.modifiers.control() =>
        {
            Some(text_editor::Binding::Custom(Message::Action(
                if key_press.modifiers.shift() {
                    text_editor::Action::Select(text_editor::Motion::End)
                } else {
                    text_editor::Action::Move(text_editor::Motion::End)
                },
            )))
        }
        iced::keyboard::Key::Character("a")
            if key_press.modifiers.control() =>
        {
            if key_press.modifiers.shift() {
                Some(text_editor::Binding::Custom(Message::Action(
                    text_editor::Action::Select(text_editor::Motion::Home),
                )))
            } else {
                Some(text_editor::Binding::Custom(Message::Action(
                    text_editor::Action::Move(text_editor::Motion::Home),
                )))
            }
        }
        iced::keyboard::Key::Character("b") if key_press.modifiers.alt() => {
            if key_press.modifiers.shift() {
                Some(text_editor::Binding::Custom(Message::Action(
                    text_editor::Action::Select(text_editor::Motion::WordLeft),
                )))
            } else {
                Some(text_editor::Binding::Custom(Message::Action(
                    text_editor::Action::Move(text_editor::Motion::WordLeft),
                )))
            }
        }
        iced::keyboard::Key::Character("b")
            if key_press.modifiers.control() =>
        {
            if key_press.modifiers.shift() {
                Some(text_editor::Binding::Custom(Message::Action(
                    text_editor::Action::Select(text_editor::Motion::Left),
                )))
            } else {
                Some(text_editor::Binding::Custom(Message::Action(
                    text_editor::Action::Move(text_editor::Motion::Left),
                )))
            }
        }
        iced::keyboard::Key::Character("f") if key_press.modifiers.alt() => {
            if key_press.modifiers.shift() {
                Some(text_editor::Binding::Custom(Message::Action(
                    text_editor::Action::Select(text_editor::Motion::WordRight),
                )))
            } else {
                Some(text_editor::Binding::Custom(Message::Action(
                    text_editor::Action::Move(text_editor::Motion::WordRight),
                )))
            }
        }
        iced::keyboard::Key::Character("f")
            if key_press.modifiers.control() =>
        {
            if key_press.modifiers.shift() {
                Some(text_editor::Binding::Custom(Message::Action(
                    text_editor::Action::Select(text_editor::Motion::Right),
                )))
            } else {
                Some(text_editor::Binding::Custom(Message::Action(
                    text_editor::Action::Move(text_editor::Motion::Right),
                )))
            }
        }
        iced::keyboard::Key::Character("d")
            if key_press.modifiers.control() =>
        {
            Some(text_editor::Binding::Custom(Message::Action(
                text_editor::Action::Edit(text_editor::Edit::Delete),
            )))
        }
        iced::keyboard::Key::Character("d") if key_press.modifiers.alt() => {
            Some(text_editor::Binding::Custom(Message::DeleteWordForward(
                true,
            )))
        }
        iced::keyboard::Key::Character("k")
            if key_press.modifiers.control() =>
        {
            Some(text_editor::Binding::Custom(Message::DeleteToEnd(true)))
        }
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn platform_specific_key_bindings(
    key_press: text_editor::KeyPress,
    selection: Option<&str>,
) -> Option<text_editor::Binding<Message>> {
    match key_press.key.as_ref() {
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Backspace)
            if key_press.modifiers.alt() && selection.is_none() =>
        {
            Some(text_editor::Binding::Custom(Message::DeleteWordBackward(
                false,
            )))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Backspace)
            if key_press.modifiers.logo() && selection.is_none() =>
        {
            Some(text_editor::Binding::Custom(Message::DeleteToStart(false)))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete)
            if key_press.modifiers.alt() =>
        {
            Some(text_editor::Binding::Custom(Message::DeleteWordForward(
                false,
            )))
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete)
            if key_press.modifiers.logo() =>
        {
            Some(text_editor::Binding::Custom(Message::DeleteToEnd(false)))
        }

        _ => None,
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_specific_key_bindings(
    key_press: text_editor::KeyPress,
    selection: Option<&str>,
) -> Option<text_editor::Binding<Message>> {
    match key_press.key.as_ref() {
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Backspace)
            if key_press.modifiers.control() && selection.is_none() =>
        {
            if key_press.modifiers.shift() {
                Some(text_editor::Binding::Custom(Message::DeleteToStart(
                    false,
                )))
            } else {
                Some(text_editor::Binding::Custom(Message::DeleteWordBackward(
                    false,
                )))
            }
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete)
            if key_press.modifiers.control() =>
        {
            if key_press.modifiers.shift() {
                Some(text_editor::Binding::Custom(Message::DeleteToEnd(false)))
            } else {
                Some(text_editor::Binding::Custom(Message::DeleteWordForward(
                    false,
                )))
            }
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Insert)
            if key_press.modifiers.shift() && key_press.text.is_none() =>
        {
            Some(text_editor::Binding::Custom(Message::Paste))
        }

        _ => None,
    }
}

pub fn view<'a>(
    state: &'a State,
    our_user: Option<&User>,
    disabled: bool,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let style = if state.error.is_some() {
        theme::text_editor::error
    } else {
        theme::text_editor::primary
    };

    let mut text_input = text_editor(&state.input_content)
        .id(state.input_id.clone())
        .placeholder("Send message...")
        .padding([2, 4])
        .wrapping(Wrapping::WordOrGlyph)
        .height(Length::Shrink)
        .line_height(theme::line_height(&config.font))
        .style(style);

    if !disabled {
        let key_bindings = config.buffer.text_input.key_bindings.clone();

        text_input = text_input.on_action(Message::Action).key_binding(
            move |key_press| {
                if !matches!(
                    key_press.status,
                    iced::widget::text_editor::Status::Focused { .. }
                ) {
                    return None;
                }

                // Try emacs bindings first if enabled
                if matches!(key_bindings, KeyBindings::Emacs)
                    && let Some(binding) = emacs_key_binding(key_press.clone())
                {
                    return Some(binding);
                }

                // Platform specific key bindings
                if let Some(binding) = platform_specific_key_bindings(
                    key_press.clone(),
                    state.input_content.selection().as_deref(),
                ) {
                    return Some(binding);
                }

                // Handling for numpad keys: treat a numpad enter the same as
                // a normal enter; treat numpad keys as character keys when
                // numlock is on (i.e. text.is_some())
                let key = if key_press.physical_key
                    == iced::keyboard::key::Physical::Code(
                        iced::keyboard::key::Code::NumpadEnter,
                    ) {
                    Cow::Owned(iced::keyboard::Key::Named(
                        iced::keyboard::key::Named::Enter,
                    ))
                } else if is_numpad(&key_press.physical_key)
                    && let Some(text) = &key_press.text
                {
                    Cow::Owned(keyboard::Key::Character(text.clone()))
                } else {
                    Cow::Borrowed(&key_press.key)
                };

                match *key {
                    // New line
                    // TODO: Add shift+enter binding
                    // iced::keyboard::Key::Named(
                    //     iced::keyboard::key::Named::Enter,
                    // ) if key_press.modifiers.shift() => {
                    //     Some(text_editor::Binding::Enter)
                    // }
                    //
                    // Send
                    iced::keyboard::Key::Named(
                        iced::keyboard::key::Named::Enter,
                    ) => Some(text_editor::Binding::Custom(Message::Send)),
                    // Tab
                    iced::keyboard::Key::Named(
                        iced::keyboard::key::Named::Tab,
                    ) => Some(text_editor::Binding::Custom(Message::Tab(
                        key_press.modifiers.shift(),
                    ))),
                    // Up
                    iced::keyboard::Key::Named(
                        iced::keyboard::key::Named::ArrowUp,
                    ) => Some(text_editor::Binding::Custom(Message::Up)),
                    // Down
                    iced::keyboard::Key::Named(
                        iced::keyboard::key::Named::ArrowDown,
                    ) => Some(text_editor::Binding::Custom(Message::Down)),
                    // Escape
                    iced::keyboard::Key::Named(
                        iced::keyboard::key::Named::Escape,
                    ) => Some(text_editor::Binding::Custom(Message::Escape)),
                    _ => text_editor::Binding::from_key_press(key_press),
                }
            },
        );
    }

    let text_input = decorate(text_input).update(
        move |_state: &mut State,
              inner: &mut Element<'a, Message>,
              tree: &mut Tree,
              event: &iced::Event,
              layout: Layout<'_>,
              cursor: mouse::Cursor,
              renderer: &Renderer,
              clipboard: &mut dyn Clipboard,
              shell: &mut Shell<'_, Message>,
              viewport: &iced::Rectangle| {
            if let event::Event::Mouse(mouse::Event::WheelScrolled { .. }) =
                event
            {
                return;
            };

            inner.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell,
                viewport,
            );
        },
    );

    let wrapped_input: Element<'a, Message> = context_menu(
        context_menu::MouseButton::default(),
        context_menu::Anchor::Cursor,
        context_menu::ToggleBehavior::KeepOpen,
        text_input,
        Actions::list(),
        move |menu, length| {
            let context_button =
                |title: Text<'a>,
                 keybind: Option<data::shortcut::KeyBind>,
                 message: Option<Message>| {
                    button(
                        row![
                            title.line_height(theme::line_height(&config.font)),
                            keybind.map(|kb| {
                                text(format!("({kb})"))
                                    .shaping(Shaping::Advanced)
                                    .size(theme::TEXT_SIZE - 2.0)
                                    .style(theme::text::secondary)
                                    .font_maybe(
                                        theme::font_style::secondary(theme)
                                            .map(font::get),
                                    )
                            }),
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                    )
                    .width(length)
                    .padding(config.spacing.context_menu.padding.entry)
                    .on_press_maybe(message)
                    .into()
                };

            match menu {
                Actions::Cut => context_button(
                    text("Cut"),
                    Some(shortcut::cut()),
                    state.input_content.selection().map(|_| Message::Cut),
                ),
                Actions::Copy => context_button(
                    text("Copy"),
                    Some(shortcut::copy()),
                    state.input_content.selection().map(|_| Message::Copy),
                ),
                Actions::CopyAll => context_button(
                    text("Copy All"),
                    None,
                    if !state.input_content.text().is_empty() {
                        Some(Message::CopyAll)
                    } else {
                        None
                    },
                ),
                Actions::SelectAll => context_button(
                    text("Select All"),
                    Some(shortcut::select_all()),
                    if !state.input_content.text().is_empty() {
                        Some(Message::SelectAll)
                    } else {
                        None
                    },
                ),
                Actions::Paste => context_button(
                    text("Paste"),
                    Some(shortcut::paste()),
                    Some(Message::Paste),
                ),
            }
        },
    )
    .mouse_interaction_on_hover(iced::advanced::mouse::Interaction::Text)
    .into();

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
        config.buffer.text_input.nickname.enabled.then(move || {
            our_user.map(|user| {
                container(
                    text(user.display(
                        config.buffer.text_input.nickname.show_access_level,
                        None,
                    ))
                    .style(move |_| our_user_style)
                    .font_maybe(
                        theme::font_style::nickname(theme, false)
                            .map(font::get),
                    ),
                )
                .padding(padding::right(4))
            })
        });

    let maybe_vertical_rule =
        maybe_our_user.is_some().then(move || rule::vertical(1.0));

    let content = column![
        container(
            row![maybe_our_user, maybe_vertical_rule, wrapped_input]
                .spacing(4)
                .height(Length::Shrink)
                .align_y(Alignment::Center)
        )
        .max_height(
            (7.55 * theme::resolve_line_height(&config.font).ceil()).ceil(),
        )
        .padding(8)
        .style(theme::container::buffer_text_input)
    ]
    .spacing(4)
    .padding(padding::top(4));

    let overlay = column![
        state.completion.view(
            state.input_content.text().as_str(),
            config,
            theme
        ),
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
    input_id: widget::Id,
    input_content: text_editor::Content,
    error: Option<String>,
    completion: Completion,
    selected_history: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self::new(None)
    }
}

impl State {
    pub fn new(input_draft: Option<&str>) -> Self {
        Self {
            input_id: widget::Id::unique(),
            input_content: input_draft.map_or(
                text_editor::Content::new(),
                text_editor::Content::with_text,
            ),
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
        main_window: &Window,
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

                let mut history_tasks = vec![];

                if let Ok(data::input::Parsed::Input(input)) = input::parse(
                    buffer.clone(),
                    config.buffer.text_input.auto_format,
                    message.as_str(),
                    clients.nickname(buffer.server()),
                    &clients.get_isupport(buffer.server()),
                    config,
                ) {
                    if let Some(encoded) = input.encoded() {
                        clients.send(buffer, encoded, TokenPriority::User);
                    }

                    if let Some(nick) = clients.nickname(buffer.server()) {
                        let mut user = nick.to_owned().into();
                        let mut channel_users = None;

                        let chantypes = clients.get_chantypes(buffer.server());
                        let statusmsg = clients.get_statusmsg(buffer.server());
                        let casemapping =
                            clients.get_casemapping(buffer.server());
                        let supports_echoes =
                            clients.get_server_supports_echoes(buffer.server());

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

                        if let Some(messages) = input.messages(
                            user,
                            channel_users,
                            chantypes,
                            statusmsg,
                            casemapping,
                            supports_echoes,
                        ) {
                            for message in messages {
                                history_tasks.extend(
                                    history
                                        .record_input_message(
                                            message,
                                            buffer.server(),
                                            casemapping,
                                            config,
                                        )
                                        .into_iter(),
                                );
                            }
                        }
                    }
                }

                let history_task = if history_tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(history_tasks.into_iter().map(Task::future))
                };

                (
                    Task::none(),
                    Some(Event::InputSent {
                        history_task,
                        open_buffers: vec![],
                    }),
                )
            }
            Message::Send => {
                let raw_input = self.input_content.text().clone();
                let cursor_position =
                    self.input_content.cursor().position.column;

                // Reset error
                self.error = None;
                // Reset selected history
                self.selected_history = None;

                if let Some(entry) = self.completion.select(config) {
                    let chantypes = clients.get_chantypes(buffer.server());
                    let actions = entry.complete_input(
                        raw_input.as_str(),
                        cursor_position,
                        chantypes,
                        config,
                    );

                    self.on_completion(buffer, history, actions, true)
                } else if !raw_input.is_empty() {
                    self.completion.reset();

                    // Parse input
                    let input = match input::parse(
                        buffer.clone(),
                        config.buffer.text_input.auto_format,
                        raw_input.as_str(),
                        clients.nickname(buffer.server()),
                        &clients.get_isupport(buffer.server()),
                        config,
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
                                            server: buffer.server().clone(),
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
                                            data::Input::from_command(
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
                                                server: buffer.server().clone(),
                                                targets: vec![(
                                                    target,
                                                    buffer_action,
                                                )],
                                            }
                                        });

                                    return (delayed_join_task, event);
                                }
                                command::Internal::ChannelDiscovery => {
                                    self.input_content =
                                        text_editor::Content::new();
                                    return (
                                        Task::none(),
                                        Some(Event::OpenInternalBuffer(
                                            buffer::Internal::ChannelDiscovery(
                                                Some(buffer.server().clone()),
                                            ),
                                        )),
                                    );
                                }
                                command::Internal::Delay(_) => {
                                    return (Task::none(), None);
                                }
                                command::Internal::ClearBuffer => {
                                    let kind = history::Kind::from_input_buffer(
                                        buffer.clone(),
                                    );

                                    let event = history
                                        .clear_messages(kind, clients)
                                        .map(|history_task| Event::Cleared {
                                            history_task: Task::future(
                                                history_task,
                                            ),
                                        });

                                    return (Task::none(), event);
                                }
                                command::Internal::SysInfo => {
                                    self.input_content =
                                        text_editor::Content::new();

                                    return (
                                        iced::system::information()
                                            .map(Message::SysInfoReceived),
                                        None,
                                    );
                                }
                                command::Internal::Connect(server) => {
                                    self.input_content =
                                        text_editor::Content::new();

                                    return (
                                        Task::none(),
                                        Some(Event::OpenServer(server)),
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
                    self.input_content = text_editor::Content::new();

                    if let Some(encoded) = input.encoded() {
                        let sent_time = server_time(&encoded);

                        clients.send(buffer, encoded, TokenPriority::User);

                        let supports_echoes =
                            clients.get_server_supports_echoes(buffer.server());

                        if config.buffer.mark_as_read.on_message_sent
                            // If the server supports echoes, then send MARKREAD
                            // on echo only (not when recording the input)
                            && !supports_echoes
                        {
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
                        let supports_echoes =
                            clients.get_server_supports_echoes(buffer.server());

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

                        let mut history_tasks = vec![];

                        if let Some(messages) = input.messages(
                            user,
                            channel_users,
                            chantypes,
                            statusmsg,
                            casemapping,
                            supports_echoes,
                        ) {
                            for message in messages {
                                history_tasks.extend(
                                    history
                                        .record_input_message(
                                            message,
                                            buffer.server(),
                                            casemapping,
                                            config,
                                        )
                                        .into_iter(),
                                );
                            }
                        }

                        history_task = Task::batch(
                            history_tasks.into_iter().map(Task::future),
                        );
                    }

                    let open_buffers =
                        if let Some(command::Irc::Join(targets, _)) =
                            input.command()
                            && let Some(buffer_action) =
                                config.actions.buffer.join_channel
                        {
                            let chantypes =
                                clients.get_chantypes(buffer.server());
                            let statusmsg =
                                clients.get_statusmsg(buffer.server());
                            let casemapping =
                                clients.get_casemapping(buffer.server());

                            targets
                                .split(',')
                                .filter_map(|target| {
                                    let target = Target::parse(
                                        target,
                                        chantypes,
                                        statusmsg,
                                        casemapping,
                                    );

                                    matches!(target, Target::Channel(_))
                                        .then_some((target, buffer_action))
                                })
                                .collect()
                        } else {
                            vec![]
                        };

                    (
                        Task::none(),
                        Some(Event::InputSent {
                            history_task,
                            open_buffers,
                        }),
                    )
                } else {
                    (Task::none(), None)
                }
            }
            Message::Tab(reverse) => {
                let input = self.input_content.text();
                let cursor_position =
                    self.input_content.cursor().position.column;

                if let Some(entry) = self.completion.tab(reverse) {
                    let chantypes = clients.get_chantypes(buffer.server());
                    let actions = entry.complete_input(
                        input.as_str(),
                        cursor_position,
                        chantypes,
                        config,
                    );

                    self.on_completion(buffer, history, actions, true)
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
                    let last_seen = history.get_last_seen(buffer);
                    let filters = FilterChain::borrow(history.get_filters());
                    let channels = clients
                        .get_channels(buffer.server())
                        .cloned()
                        .collect::<Vec<_>>();
                    let supports_detach =
                        clients.get_server_supports_detach(buffer.server());
                    let isupport = clients.get_isupport(buffer.server());

                    self.completion.process(
                        &new_input,
                        new_input.len(),
                        clients.nickname(buffer.server()),
                        users,
                        filters,
                        &last_seen,
                        &channels,
                        current_target.as_ref(),
                        buffer.server(),
                        supports_detach,
                        &isupport,
                        config,
                    );

                    return self.on_history_navigation(
                        buffer, history, &new_input, false,
                    );
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
                        let last_seen = history.get_last_seen(buffer);
                        let filters =
                            FilterChain::borrow(history.get_filters());
                        let channels = clients
                            .get_channels(buffer.server())
                            .cloned()
                            .collect::<Vec<_>>();
                        let supports_detach =
                            clients.get_server_supports_detach(buffer.server());
                        let isupport = clients.get_isupport(buffer.server());

                        self.completion.process(
                            &new_input,
                            new_input.len(),
                            clients.nickname(buffer.server()),
                            users,
                            filters,
                            &last_seen,
                            &channels,
                            current_target.as_ref(),
                            buffer.server(),
                            supports_detach,
                            &isupport,
                            config,
                        );
                        new_input
                    };

                    return self.on_history_navigation(
                        buffer, history, &new_input, false,
                    );
                }

                (Task::none(), None)
            }
            // Capture escape so that closing context menu or commands/emojis picker
            // does not defocus input
            Message::Escape => (Task::none(), None),
            Message::SendCommand { buffer, command } => {
                let input = data::Input::from_command(buffer.clone(), command)
                    .encoded();

                // Send command.
                if let Some(input) = input {
                    clients.send(&buffer, input, TokenPriority::User);
                }

                (Task::none(), None)
            }
            Message::Paste => {
                let task = clipboard::read().and_then(|clipboard| {
                    Task::done(Message::Action(text_editor::Action::Edit(
                        text_editor::Edit::Paste(std::sync::Arc::new(
                            clipboard,
                        )),
                    )))
                });

                Self::close_context_menu(main_window.id, vec![task])
            }
            Message::Cut => {
                let task =
                    if let Some(selection) = self.input_content.selection() {
                        self.input_content.perform(text_editor::Action::Edit(
                            text_editor::Edit::Delete,
                        ));

                        clipboard::write(selection.to_string())
                    } else {
                        Task::none()
                    };

                Self::close_context_menu(main_window.id, vec![task])
            }
            Message::Copy => {
                let task = if let Some(input) = self.input_content.selection() {
                    clipboard::write(input.to_string())
                } else {
                    Task::none()
                };

                Self::close_context_menu(main_window.id, vec![task])
            }
            Message::CopyAll => {
                let input = self.input_content.text();
                let task = clipboard::write(input.to_string());

                Self::close_context_menu(main_window.id, vec![task])
            }
            Message::SelectAll => {
                self.input_content.perform(text_editor::Action::SelectAll);

                Self::close_context_menu(main_window.id, vec![])
            }
            Message::CloseContextMenu(_, _) => (Task::none(), None),
            Message::DeleteWordBackward(save_to_clipboard) => {
                self.input_content.perform(text_editor::Action::Select(
                    text_editor::Motion::WordLeft,
                ));

                let task = if save_to_clipboard {
                    self.input_content.selection().map_or_else(
                        Task::none,
                        |selection| {
                            let text = selection.to_string();

                            clipboard::write(text)
                        },
                    )
                } else {
                    Task::none()
                };

                self.input_content.perform(text_editor::Action::Edit(
                    text_editor::Edit::Delete,
                ));

                (task, None)
            }
            Message::DeleteWordForward(save_to_clipboard) => {
                self.input_content.perform(text_editor::Action::Select(
                    text_editor::Motion::WordRight,
                ));

                let task = if save_to_clipboard {
                    self.input_content.selection().map_or_else(
                        Task::none,
                        |selection| {
                            let text = selection.to_string();

                            clipboard::write(text)
                        },
                    )
                } else {
                    Task::none()
                };

                self.input_content.perform(text_editor::Action::Edit(
                    text_editor::Edit::Delete,
                ));

                (task, None)
            }
            Message::DeleteToEnd(save_to_clipboard) => {
                self.input_content.perform(text_editor::Action::Select(
                    text_editor::Motion::End,
                ));

                let task = if save_to_clipboard {
                    self.input_content.selection().map_or_else(
                        Task::none,
                        |selection| {
                            let text = selection.to_string();
                            clipboard::write(text)
                        },
                    )
                } else {
                    Task::none()
                };

                self.input_content.perform(text_editor::Action::Edit(
                    text_editor::Edit::Delete,
                ));

                (task, None)
            }
            Message::DeleteToStart(save_to_clipboard) => {
                self.input_content.perform(text_editor::Action::Select(
                    text_editor::Motion::Home,
                ));

                let task = if save_to_clipboard {
                    self.input_content.selection().map_or_else(
                        Task::none,
                        |selection| {
                            let text = selection.to_string();
                            clipboard::write(text)
                        },
                    )
                } else {
                    Task::none()
                };

                self.input_content.perform(text_editor::Action::Edit(
                    text_editor::Edit::Delete,
                ));

                (task, None)
            }
            Message::Action(action) => {
                if let text_editor::Action::Edit(text_editor::Edit::Paste(
                    clipboard,
                )) = &action
                {
                    // TODO: Remove newline cleaning when adding multiline
                    // support
                    let cleaned = clipboard.replace(['\n', '\r'], " ");
                    let action = text_editor::Action::Edit(
                        text_editor::Edit::Paste(std::sync::Arc::new(cleaned)),
                    );
                    self.input_content.perform(action);
                } else {
                    self.input_content.perform(action.clone());
                }

                match &action {
                    text_editor::Action::Edit(_) => {
                        let input = self.input_content.text();
                        let cursor_position =
                            self.input_content.cursor().position.column;

                        // Reset error state
                        self.error = None;
                        // Reset selected history
                        self.selected_history = None;

                        let users = buffer.channel().and_then(|channel| {
                            clients.get_channel_users(buffer.server(), channel)
                        });
                        let last_seen = history.get_last_seen(buffer);
                        let filters =
                            FilterChain::borrow(history.get_filters());
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
                            cursor_position,
                            clients.nickname(buffer.server()),
                            users,
                            filters,
                            &last_seen,
                            &channels,
                            current_target.as_ref(),
                            buffer.server(),
                            supports_detach,
                            &isupport,
                            config,
                        );

                        let actions = self
                            .completion
                            .complete_emoji(&input, cursor_position);

                        if let Some(actions) = actions {
                            for action in actions.into_iter() {
                                self.input_content.perform(action);
                            }
                        }

                        if let Err(error) = input::parse(
                            buffer.clone(),
                            config.buffer.text_input.auto_format,
                            &input,
                            clients.nickname(buffer.server()),
                            &clients.get_isupport(buffer.server()),
                            config,
                        ) && match error {
                            input::Error::ExceedsByteLimit { .. } => true,
                            input::Error::Command(
                                command::Error::IncorrectArgCount {
                                    actual,
                                    max,
                                    ..
                                },
                            ) => actual > max,
                            input::Error::Command(
                                command::Error::MissingSlash,
                            ) => false,
                            input::Error::Command(
                                command::Error::MissingCommand,
                            ) => false,
                            input::Error::Command(
                                command::Error::NoModeString,
                            ) => false,
                            input::Error::Command(
                                command::Error::InvalidModeString,
                            ) => true,
                            input::Error::Command(
                                command::Error::ArgTooLong { .. },
                            ) => true,
                            input::Error::Command(
                                command::Error::TooManyTargets { .. },
                            ) => true,
                            input::Error::Command(
                                command::Error::NotPositiveInteger,
                            ) => true,
                            input::Error::Command(
                                command::Error::InvalidChannelName { .. },
                            ) => true,
                            input::Error::Command(
                                command::Error::InvalidServerUrl,
                            ) => true,
                        } {
                            self.error = Some(error.to_string());
                        }

                        history.record_draft(RawInput {
                            buffer: buffer.clone(),
                            text: input,
                        });

                        (Task::none(), None)
                    }
                    text_editor::Action::Move(_)
                    | text_editor::Action::Click(_) => {
                        let input = self.input_content.text();
                        let cursor_position =
                            self.input_content.cursor().position.column;

                        let users = buffer.channel().and_then(|channel| {
                            clients.get_channel_users(buffer.server(), channel)
                        });
                        let last_seen = history.get_last_seen(buffer);
                        let filters =
                            FilterChain::borrow(history.get_filters());
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
                            cursor_position,
                            clients.nickname(buffer.server()),
                            users,
                            filters,
                            &last_seen,
                            &channels,
                            current_target.as_ref(),
                            buffer.server(),
                            supports_detach,
                            &isupport,
                            config,
                        );

                        (Task::none(), None)
                    }
                    _ => (Task::none(), None),
                }
            }
        }
    }

    fn close_context_menu(
        window: window::Id,
        tasks: Vec<Task<Message>>,
    ) -> (Task<Message>, Option<Event>) {
        (
            Task::batch(
                vec![context_menu::close(convert::identity).map(
                    move |any_closed| {
                        Message::CloseContextMenu(window, any_closed)
                    },
                )]
                .into_iter()
                .chain(tasks)
                .collect::<Vec<_>>(),
            ),
            None,
        )
    }

    fn on_completion(
        &mut self,
        buffer: &buffer::Upstream,
        history: &mut history::Manager,
        actions: Vec<text_editor::Action>,
        record_draft: bool,
    ) -> (Task<Message>, Option<Event>) {
        for action in actions.into_iter() {
            self.input_content.perform(action);
        }

        if record_draft {
            history.record_draft(RawInput {
                buffer: buffer.clone(),
                text: self.input_content.text(),
            });
        }

        (Task::none(), None)
    }

    fn on_history_navigation(
        &mut self,
        buffer: &buffer::Upstream,
        history: &mut history::Manager,
        text: &str,
        record_draft: bool,
    ) -> (Task<Message>, Option<Event>) {
        if record_draft {
            history.record_draft(RawInput {
                buffer: buffer.clone(),
                text: text.to_string(),
            });
        }

        // update the input content
        self.input_content = text_editor::Content::with_text(text);
        // move the cursor to the end of the input
        self.input_content
            .perform(text_editor::Action::Move(text_editor::Motion::End));

        (Task::none(), None)
    }

    pub fn focus(&self) -> Task<Message> {
        let input_id = self.input_id.clone();

        operation::is_focused(input_id.clone()).then(move |is_focused| {
            if is_focused {
                Task::none()
            } else {
                operation::focus(input_id.clone())
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
    ) {
        let text = self.input_content.text();
        let cursor_position = self.input_content.cursor().position.column;

        let insert_text = if cursor_position == 0 {
            let suffix_range = cursor_position
                ..cursor_position + autocomplete.completion_suffixes[0].len();

            if text
                .get(suffix_range)
                .is_some_and(|text| text == autocomplete.completion_suffixes[0])
            {
                format!("{nick}")
            } else {
                format!("{nick}{}", autocomplete.completion_suffixes[0])
            }
        } else {
            let suffix_range = cursor_position
                ..cursor_position + autocomplete.completion_suffixes[1].len();

            if text
                .chars()
                .nth(cursor_position - 1)
                .is_some_and(|c| c == ' ')
            {
                if text.get(suffix_range).is_some_and(|text| {
                    text == autocomplete.completion_suffixes[1]
                }) {
                    format!("{nick}")
                } else {
                    format!("{nick}{}", autocomplete.completion_suffixes[1])
                }
            } else if text
                .get(suffix_range)
                .is_some_and(|text| text == autocomplete.completion_suffixes[1])
            {
                format!(" {nick}")
            } else {
                format!(" {nick}{}", autocomplete.completion_suffixes[1])
            }
        };

        self.input_content.perform(text_editor::Action::Edit(
            text_editor::Edit::Paste(std::sync::Arc::new(insert_text)),
        ));

        history.record_draft(RawInput {
            buffer,
            text: self.input_content.text(),
        });
    }

    pub fn close_picker(&mut self) -> bool {
        self.completion.close_picker()
    }
}
