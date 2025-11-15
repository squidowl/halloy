use std::collections::{HashMap, HashSet, VecDeque, hash_map};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{convert, slice};

use chrono::format::SecondsFormat;
use chrono::{DateTime, Local, Utc};
use data::config::buffer::{ScrollPosition, UsernameFormat};
use data::dashboard::{self, BufferAction};
use data::environment::{RELEASE_WEBSITE, WIKI_WEBSITE};
use data::history::ReadMarker;
use data::history::filter::Filter;
use data::isupport::{self, ChatHistorySubcommand, MessageReference};
use data::message::{self, Broadcast};
use data::rate_limit::TokenPriority;
use data::target::{self, Target};
use data::{
    Config, Notification, Server, User, Version, client, command, config,
    environment, file_transfer, history, preview, server,
};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{Space, column, container, row};
use iced::{Length, Size, Task, Vector, advanced, clipboard};
use irc::proto;

use self::command_bar::CommandBar;
use self::pane::Pane;
use self::sidebar::Sidebar;
use self::theme_editor::ThemeEditor;
use crate::buffer::{self, Buffer};
use crate::widget::{
    Column, Element, Row, anchored_overlay, context_menu, selectable_text,
    shortcut,
};
use crate::window::Window;
use crate::{Theme, event, notification, theme, window};

mod command_bar;
pub mod pane;
pub mod sidebar;
mod theme_editor;

const FOCUS_HISTORY_LEN: usize = 8;
const SAVE_AFTER: Duration = Duration::from_secs(3);

pub struct Dashboard {
    panes: Panes,
    focus: Focus,
    focus_history: VecDeque<pane_grid::Pane>,
    side_menu: Sidebar,
    history: history::Manager,
    last_changed: Option<Instant>,
    command_bar: Option<CommandBar>,
    file_transfers: file_transfer::Manager,
    theme_editor: Option<ThemeEditor>,
    notifications: notification::Notifications,
    previews: preview::Collection,
    buffer_settings: dashboard::BufferSettings,
}

#[derive(Debug)]
pub enum Message {
    Pane(window::Id, pane::Message),
    Sidebar(sidebar::Message),
    SelectedText(Vec<(f32, String)>, advanced::clipboard::Kind),
    History(history::manager::Message),
    DashboardSaved(Result<(), data::dashboard::Error>),
    Task(command_bar::Message),
    Shortcut(shortcut::Command),
    FileTransfer(file_transfer::task::Update),
    SendFileSelected(Server, User, Option<PathBuf>),
    CloseContextMenu(window::Id, bool),
    ThemeEditor(theme_editor::Message),
    ConfigReloaded(Result<Config, config::Error>),
    Client(client::Message),
    LoadPreview((url::Url, Result<data::Preview, data::preview::LoadError>)),
    NewWindow(window::Id, Pane),
}

#[derive(Debug)]
pub enum Event {
    ConfigReloaded(Result<Config, config::Error>),
    ReloadThemes,
    QuitServer(Server, Option<String>),
    IrcError(anyhow::Error),
    Exit,
    OpenUrl(String, bool),
    ImagePreview(PathBuf, url::Url),
}

impl Dashboard {
    pub fn empty(
        main_window: &Window,
        config: &Config,
    ) -> (Self, Task<Message>) {
        let (main_panes, pane) =
            pane_grid::State::new(Pane::new(Buffer::Empty));

        let mut dashboard = Dashboard {
            panes: Panes {
                main_window: main_window.id,
                main: main_panes,
                popout: HashMap::new(),
            },
            focus: Focus {
                window: main_window.id,
                pane,
            },
            focus_history: VecDeque::new(),
            side_menu: Sidebar::new(),
            history: history::Manager::default(),
            last_changed: None,
            command_bar: None,
            file_transfers: file_transfer::Manager::default(),
            theme_editor: None,
            notifications: notification::Notifications::new(config),
            previews: preview::Collection::default(),
            buffer_settings: dashboard::BufferSettings::default(),
        };

        let command = dashboard.track(None);

        (dashboard, command)
    }

    pub fn restore(
        dashboard: data::Dashboard,
        config: &Config,
        main_window: &Window,
    ) -> (Self, Task<Message>) {
        let (mut dashboard, task) =
            Dashboard::from_data(dashboard, config, main_window);

        let tasks = Task::batch(vec![task, dashboard.track(None)]);

        (dashboard, tasks)
    }

    pub fn init_filters(
        &mut self,
        servers: &server::Map,
        clients: &client::Map,
    ) {
        self.history
            .set_filters(Filter::list_from_servers(servers, clients));
    }

    pub fn update_filters(
        &mut self,
        servers: &server::Map,
        clients: &client::Map,
        buffer_config: &config::Buffer,
    ) {
        self.init_filters(servers, clients);

        self.reprocess_history(clients, buffer_config);
    }

    pub fn reprocess_history(
        &mut self,
        clients: &client::Map,
        buffer_config: &config::Buffer,
    ) {
        let open_pane_kinds: Vec<history::Kind> = self
            .panes
            .iter()
            .filter_map(|(_window_id, _grid_pane, pane)| {
                if matches!(
                    pane.buffer,
                    Buffer::Channel(_)
                        | Buffer::Server(_)
                        | Buffer::Query(_)
                        | Buffer::Highlights(_)
                ) {
                    pane.buffer.data().and_then(history::Kind::from_buffer)
                } else {
                    None
                }
            })
            .collect();

        open_pane_kinds.into_iter().for_each(|kind| {
            self.history.process_messages(kind, clients, buffer_config);
        });
    }

    pub fn renormalize_history(
        &mut self,
        server: &data::Server,
        clients: &client::Map,
    ) {
        let open_pane_kinds: Vec<history::Kind> = self
            .panes
            .iter()
            .filter_map(|(_window_id, _grid_pane, pane)| {
                if pane
                    .buffer
                    .server()
                    .is_some_and(|buffer_server| buffer_server == *server)
                    || matches!(pane.buffer, Buffer::Highlights(_))
                {
                    pane.buffer.data().and_then(history::Kind::from_buffer)
                } else {
                    None
                }
            })
            .collect();

        open_pane_kinds.into_iter().for_each(|kind| {
            self.history.renormalize_messages(&kind, clients);
        });
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut client::Map,
        theme: &mut Theme,
        version: &Version,
        config: &Config,
        main_window: &Window,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Pane(window, message) => {
                match message {
                    pane::Message::PaneClicked(pane) => {
                        return (self.focus_pane(window, pane), None);
                    }
                    pane::Message::PaneResized(pane_grid::ResizeEvent {
                        split,
                        ratio,
                    }) => {
                        // Pane grid interactions only enabled for main window panegrid
                        self.panes.main.resize(split, ratio);
                        self.last_changed = Some(Instant::now());
                    }
                    pane::Message::PaneDragged(
                        pane_grid::DragEvent::Dropped { pane, target },
                    ) => {
                        // Pane grid interactions only enabled for main window panegrid
                        self.panes.main.drop(pane, target);
                        self.last_changed = Some(Instant::now());
                    }
                    pane::Message::PaneDragged(_) => {}
                    pane::Message::ClosePane => {
                        return (
                            self.close_pane(
                                clients,
                                config,
                                self.focus.window,
                                self.focus.pane,
                            ),
                            None,
                        );
                    }
                    pane::Message::SplitPane(axis) => {
                        return (self.split_pane(axis), None);
                    }
                    pane::Message::Buffer(id, message) => {
                        if let Some(pane) = self.panes.get_mut(window, id) {
                            let (command, event) = pane.buffer.update(
                                message,
                                clients,
                                &mut self.history,
                                &mut self.file_transfers,
                                config,
                            );

                            let task = command.map(move |message| {
                                Message::Pane(
                                    window,
                                    pane::Message::Buffer(id, message),
                                )
                            });

                            let Some(event) = event else {
                                return (task, None);
                            };

                            let (buffer_task, buffer_event) = self
                                .handle_buffer_event(
                                    window, id, event, clients, config,
                                );

                            return (
                                Task::batch(vec![task, buffer_task]),
                                buffer_event,
                            );
                        }
                    }
                    pane::Message::ToggleShowUserList => {
                        if let Some((_, _, pane)) = self.get_focused_mut() {
                            if let Some(buffer) = pane.buffer.data() {
                                let settings = self.buffer_settings.entry(
                                    &buffer,
                                    Some(config.buffer.clone().into()),
                                );
                                settings.channel.nicklist.toggle_visibility();
                            }

                            self.last_changed = Some(Instant::now());
                            return (Task::none(), None);
                        }
                    }
                    pane::Message::ToggleShowTopic => {
                        if let Some((_, _, pane)) = self.get_focused_mut() {
                            if let Some(buffer) = pane.buffer.data() {
                                let settings = self.buffer_settings.entry(
                                    &buffer,
                                    Some(config.buffer.clone().into()),
                                );
                                settings
                                    .channel
                                    .topic_banner
                                    .toggle_visibility();
                            }

                            self.last_changed = Some(Instant::now());
                            return (Task::none(), None);
                        }
                    }
                    pane::Message::MaximizePane => self.maximize_pane(),
                    pane::Message::Popout => {
                        return (self.popout_pane(clients, config), None);
                    }
                    pane::Message::Merge => {
                        return (self.merge_pane(clients, config), None);
                    }
                    pane::Message::ScrollToBottom => {
                        let Focus { window, pane } = self.focus;

                        if let Some(state) = self.panes.get_mut(window, pane) {
                            let mut task = state
                                .buffer
                                .scroll_to_end(config)
                                .map(move |message| {
                                    Message::Pane(
                                        window,
                                        pane::Message::Buffer(pane, message),
                                    )
                                });

                            if config.buffer.mark_as_read.on_scroll_to_bottom {
                                task = task.chain(Task::done(Message::Pane(
                                    window,
                                    pane::Message::MarkAsRead,
                                )));
                            }

                            return (task, None);
                        }
                    }
                    pane::Message::MarkAsRead => {
                        if let Some((_, _, pane)) = self.get_focused_mut()
                            && let Some(kind) = pane
                                .buffer
                                .data()
                                .and_then(history::Kind::from_buffer)
                        {
                            mark_as_read(
                                kind,
                                &mut self.history,
                                clients,
                                TokenPriority::User,
                            );
                        }
                    }
                    pane::Message::ContentResized(id, size) => {
                        if let Some(state) = self.panes.get_mut(window, id) {
                            state.size = size;
                            state.buffer.update_pane_size(size, config);
                        }
                    }
                }
            }
            Message::Sidebar(message) => {
                let (command, event) = self.side_menu.update(message);

                let Some(event) = event else {
                    return (command.map(Message::Sidebar), None);
                };

                let (event_task, event) = match event {
                    sidebar::Event::CloseAllQueries(server, queries) => (
                        self.leave_all_queries(
                            clients, config, server, queries,
                        ),
                        None,
                    ),
                    sidebar::Event::QuitApplication => {
                        (self.exit(clients, config), None)
                    }
                    sidebar::Event::New(buffer) => (
                        self.open_buffer(
                            data::Buffer::Upstream(buffer),
                            BufferAction::NewPane,
                            clients,
                            config,
                        ),
                        None,
                    ),
                    sidebar::Event::Popout(buffer) => (
                        self.open_buffer(
                            data::Buffer::Upstream(buffer),
                            BufferAction::NewWindow,
                            clients,
                            config,
                        ),
                        None,
                    ),
                    sidebar::Event::Focus(window, pane) => {
                        (self.focus_pane(window, pane), None)
                    }
                    sidebar::Event::Replace(buffer) => (
                        self.open_buffer(
                            data::Buffer::Upstream(buffer),
                            BufferAction::ReplacePane,
                            clients,
                            config,
                        ),
                        None,
                    ),
                    sidebar::Event::Close(window, pane) => {
                        (self.close_pane(clients, config, window, pane), None)
                    }
                    sidebar::Event::Swap(window, pane) => {
                        (self.swap_pane_with_focus(window, pane), None)
                    }
                    sidebar::Event::Detach(buffer) => {
                        if let Some(target) = buffer.target() {
                            let server = buffer.server();

                            (
                                self.leave_server_target(
                                    clients,
                                    config,
                                    server.clone(),
                                    target,
                                    Some("detach".to_string()),
                                ),
                                None,
                            )
                        } else {
                            (Task::none(), None)
                        }
                    }
                    sidebar::Event::Leave(buffer) => {
                        self.leave_buffer(clients, config, buffer)
                    }
                    sidebar::Event::ToggleInternalBuffer(buffer) => (
                        self.toggle_internal_buffer(clients, config, buffer),
                        None,
                    ),
                    sidebar::Event::ToggleCommandBar => (
                        self.toggle_command_bar(
                            &closed_buffers(self, clients),
                            version,
                            config,
                            theme,
                        ),
                        None,
                    ),
                    sidebar::Event::ConfigReloaded(conf) => {
                        (Task::none(), Some(Event::ConfigReloaded(conf)))
                    }
                    sidebar::Event::OpenReleaseWebsite => {
                        let _ = open::that_detached(RELEASE_WEBSITE);
                        (Task::none(), None)
                    }
                    sidebar::Event::ToggleThemeEditor => (
                        self.toggle_theme_editor(theme, main_window, config),
                        None,
                    ),
                    sidebar::Event::OpenDocumentation => {
                        let _ = open::that_detached(WIKI_WEBSITE);
                        (Task::none(), None)
                    }
                    sidebar::Event::MarkServerAsRead(server) => {
                        mark_server_as_read(server, &mut self.history, clients);

                        (Task::none(), None)
                    }
                    sidebar::Event::MarkAsRead(buffer) => {
                        if let Some(kind) = history::Kind::from_buffer(
                            data::Buffer::Upstream(buffer),
                        ) {
                            mark_as_read(
                                kind,
                                &mut self.history,
                                clients,
                                TokenPriority::User,
                            );
                        }

                        (Task::none(), None)
                    }
                    sidebar::Event::OpenConfigFile => {
                        let _ = open::that_detached(Config::path());
                        (Task::none(), None)
                    }
                };

                let window = main_window.id;

                return (
                    Task::batch(vec![
                        context_menu::close(convert::identity).map(
                            move |any_closed| {
                                Message::CloseContextMenu(window, any_closed)
                            },
                        ),
                        event_task,
                        command.map(Message::Sidebar),
                    ]),
                    event,
                );
            }
            Message::SelectedText(contents, clipboard_kind) => {
                let mut last_y = None;
                let contents = contents.into_iter().fold(
                    String::new(),
                    |acc, (y, content)| {
                        if let Some(_y) = last_y {
                            let new_line = if y == _y { "" } else { "\n" };
                            last_y = Some(y);

                            format!("{acc}{new_line}{content}")
                        } else {
                            last_y = Some(y);

                            content
                        }
                    },
                );

                if !contents.is_empty() {
                    return (
                        match clipboard_kind {
                            advanced::clipboard::Kind::Standard => {
                                clipboard::write(contents)
                            }
                            advanced::clipboard::Kind::Primary => {
                                clipboard::write_primary(contents)
                            }
                        },
                        None,
                    );
                }
            }
            Message::History(message) => {
                if let Some(event) =
                    self.history.update(message, clients, &config.buffer)
                {
                    match event {
                        history::manager::Event::Loaded(kind) => {
                            let buffer = kind.into();

                            if let Some((window, pane, state)) =
                                self.panes.get_mut_by_buffer(&buffer)
                            {
                                return (
                                    match config.buffer.scroll_position_on_open
                                    {
                                        ScrollPosition::OldestUnread => state
                                            .buffer
                                            .scroll_to_backlog(
                                                &self.history,
                                                config,
                                            )
                                            .map(move |message| {
                                                Message::Pane(
                                                    window,
                                                    pane::Message::Buffer(
                                                        pane, message,
                                                    ),
                                                )
                                            }),
                                        ScrollPosition::Newest => Task::none(),
                                    },
                                    None,
                                );
                            }
                        }
                        history::manager::Event::Exited => {
                            return (Task::none(), Some(Event::Exit));
                        }
                        history::manager::Event::SentMessageUpdated(
                            kind,
                            read_marker,
                        ) => {
                            if config.buffer.mark_as_read.on_message_sent
                                && let (Some(server), Some(target)) =
                                    (kind.server(), kind.target())
                            {
                                clients.send_markread(
                                    server,
                                    target,
                                    read_marker,
                                    TokenPriority::High,
                                );
                            }
                        }
                        history::manager::Event::ResendMessage(
                            kind,
                            message,
                        ) => {
                            if let Some(buffer) =
                                data::Buffer::from(kind).upstream()
                                && let Some(user) = clients
                                    .nickname(buffer.server())
                                    .map(|nick| User::from(nick.to_owned()))
                                && let Some(command) = message.command
                            {
                                let (user, channel_users) =
                                    if let buffer::Upstream::Channel(
                                        server,
                                        channel,
                                    ) = &buffer
                                    {
                                        (
                                            clients
                                                .resolve_user_attributes(
                                                    server, channel, &user,
                                                )
                                                .cloned()
                                                .unwrap_or(user),
                                            clients.get_channel_users(
                                                server, channel,
                                            ),
                                        )
                                    } else {
                                        (user, None)
                                    };
                                let chantypes =
                                    clients.get_chantypes(buffer.server());
                                let statusmsg =
                                    clients.get_statusmsg(buffer.server());
                                let casemapping =
                                    clients.get_casemapping(buffer.server());
                                let supports_echoes = clients
                                    .get_server_supports_echoes(
                                        buffer.server(),
                                    );

                                if let Some(messages) = command.messages(
                                    user,
                                    channel_users,
                                    chantypes,
                                    statusmsg,
                                    casemapping,
                                    supports_echoes,
                                    config,
                                ) && let Some(encoded) =
                                    proto::Command::try_from(command)
                                        .ok()
                                        .map(proto::Message::from)
                                        .map(message::Encoded::from)
                                {
                                    clients.send(
                                        buffer,
                                        encoded,
                                        TokenPriority::User,
                                    );

                                    return (
                                        Task::batch(
                                            messages
                                                .into_iter()
                                                .flat_map(|message| {
                                                    self.history
                                                        .record_input_message(
                                                            message,
                                                            buffer.server(),
                                                            casemapping,
                                                            config,
                                                        )
                                                })
                                                .map(Task::future),
                                        )
                                        .map(Message::History),
                                        None,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Message::DashboardSaved(Ok(())) => {
                log::debug!("dashboard saved");
            }
            Message::DashboardSaved(Err(error)) => {
                log::warn!("error saving dashboard: {error}");
            }
            Message::Task(message) => {
                let Some(command_bar) = &mut self.command_bar else {
                    return (Task::none(), None);
                };

                match command_bar.update(message) {
                    Some(command_bar::Event::ThemePreview(preview)) => {
                        match preview {
                            Some(preview) => *theme = theme.preview(preview),
                            None => *theme = theme.selected(),
                        }
                    }
                    Some(command_bar::Event::Command(command)) => {
                        let (command, event) = match command {
                            command_bar::Command::Version(command) => match command {
                                command_bar::Version::Application(_) => {
                                    let _ = open::that_detached(RELEASE_WEBSITE);
                                    (Task::none(), None)
                                }
                            },
                            command_bar::Command::Buffer(command) => match command {
                                command_bar::Buffer::Maximize(_) => {
                                    self.maximize_pane();
                                    (Task::none(), None)
                                }
                                command_bar::Buffer::New => {
                                    (self.new_pane(pane_grid::Axis::Horizontal), None)
                                }
                                command_bar::Buffer::Close => {
                                    let Focus { window, pane } = self.focus;
                                    (self.close_pane(clients, config, window, pane), None)
                                }
                                command_bar::Buffer::Replace(buffer) => (
                                    self.open_buffer(
                                        data::Buffer::Upstream(buffer),
                                        BufferAction::ReplacePane,
                                        clients,
                                        config,
                                    ),
                                    None,
                                ),
                                command_bar::Buffer::Popout => (self.popout_pane(clients, config), None),
                                command_bar::Buffer::Merge => (self.merge_pane(clients, config), None),
                                command_bar::Buffer::ToggleInternal(buffer) => {
                                    (self.toggle_internal_buffer(clients, config, buffer), None)
                                }
                            },
                            command_bar::Command::Configuration(command) => match command {
                                command_bar::Configuration::OpenConfigDirectory => {
                                    let _ = open::that_detached(Config::config_dir());
                                    (Task::none(), None)
                                }
                                command_bar::Configuration::OpenCacheDirectory => {
                                    let _ = open::that_detached(environment::cache_dir());
                                    (Task::none(), None)
                                }
                                command_bar::Configuration::OpenDataDirectory => {
                                    let _ = open::that_detached(environment::data_dir());
                                    (Task::none(), None)
                                }
                                command_bar::Configuration::OpenWebsite => {
                                    let _ = open::that_detached(environment::WIKI_WEBSITE);
                                    (Task::none(), None)
                                }
                                command_bar::Configuration::Reload => {
                                    (Task::perform(Config::load(), Message::ConfigReloaded), None)
                                }
                                command_bar::Configuration::OpenConfigFile => {
                                    let _ = open::that_detached(Config::path());
                                    (Task::none(), None)
                                },
                            },
                            command_bar::Command::Theme(command) => match command {
                                command_bar::Theme::Switch(new) => {
                                    *theme = Theme::from(new);
                                    (Task::none(), None)
                                }
                                command_bar::Theme::OpenEditor => {
                                    if let Some(editor) = &self.theme_editor {
                                        (window::gain_focus(editor.window), None)
                                    } else {
                                        let (editor, task) = ThemeEditor::open(main_window, config);

                                        self.theme_editor = Some(editor);

                                        (task.then(|_| Task::none()), None)
                                    }
                                }
                                command_bar::Theme::OpenThemesWebsite => {
                                    let _ = open::that_detached(environment::THEME_WEBSITE);
                                    (Task::none(), None)
                                }
                            },
                            command_bar::Command::Application(application) => match application {
                                command_bar::Application::Quit => (self.exit(clients, config), None),
                                command_bar::Application::ToggleFullscreen => (window::toggle_fullscreen(), None),
                                command_bar::Application::ToggleSidebarVisibility => {
                                    self.side_menu.toggle_visibility();
                                    (Task::none(), None)
                                }
                            },
                        };

                        return (
                            Task::batch(vec![
                                command,
                                self.toggle_command_bar(
                                    &closed_buffers(self, clients),
                                    version,
                                    config,
                                    theme,
                                ),
                            ]),
                            event,
                        );
                    }
                    Some(command_bar::Event::Unfocused) => {
                        return (
                            self.toggle_command_bar(
                                &closed_buffers(self, clients),
                                version,
                                config,
                                theme,
                            ),
                            None,
                        );
                    }
                    None => {}
                }
            }
            Message::Shortcut(shortcut) => {
                use shortcut::Command::*;

                // Only works on main window / pane_grid
                let mut move_focus = |direction: pane_grid::Direction| {
                    let Focus { window, pane } = self.focus;

                    if window == self.main_window()
                        && let Some(adjacent) =
                            self.panes.main.adjacent(pane, direction)
                    {
                        return self.focus_pane(window, adjacent);
                    }

                    Task::none()
                };

                match shortcut {
                    MoveUp => {
                        return (move_focus(pane_grid::Direction::Up), None);
                    }
                    MoveDown => {
                        return (move_focus(pane_grid::Direction::Down), None);
                    }
                    MoveLeft => {
                        return (move_focus(pane_grid::Direction::Left), None);
                    }
                    MoveRight => {
                        return (move_focus(pane_grid::Direction::Right), None);
                    }
                    CloseBuffer => {
                        let Focus { window, pane } = self.focus;
                        return (
                            self.close_pane(clients, config, window, pane),
                            None,
                        );
                    }
                    MaximizeBuffer => {
                        let Focus { window, pane } = self.focus;
                        // Only main window has >1 pane to maximize
                        if window == self.main_window() {
                            self.panes.main.maximize(pane);
                        }
                    }
                    RestoreBuffer => {
                        self.panes.main.restore();
                    }
                    CycleNextBuffer => {
                        let all_buffers = all_buffers(clients, &self.history);
                        let open_buffers = open_buffers(self);

                        if let Some((window, pane, state, history)) =
                            self.get_focused_with_history_mut()
                            && let Some(buffer) = cycle_next_buffer(
                                state.buffer.upstream(),
                                all_buffers,
                                &open_buffers,
                            )
                        {
                            mark_as_read_on_buffer_close(
                                &state.buffer,
                                history,
                                clients,
                                config,
                            );

                            state.buffer = Buffer::from_data(
                                data::Buffer::Upstream(buffer),
                                state.size,
                                config,
                            );
                            self.last_changed = Some(Instant::now());
                            return (self.focus_pane(window, pane), None);
                        }
                    }
                    CyclePreviousBuffer => {
                        let all_buffers = all_buffers(clients, &self.history);
                        let open_buffers = open_buffers(self);

                        if let Some((window, pane, state, history)) =
                            self.get_focused_with_history_mut()
                            && let Some(buffer) = cycle_previous_buffer(
                                state.buffer.upstream(),
                                all_buffers,
                                &open_buffers,
                            )
                        {
                            mark_as_read_on_buffer_close(
                                &state.buffer,
                                history,
                                clients,
                                config,
                            );

                            state.buffer = Buffer::from_data(
                                data::Buffer::Upstream(buffer),
                                state.size,
                                config,
                            );
                            self.last_changed = Some(Instant::now());
                            return (self.focus_pane(window, pane), None);
                        }
                    }
                    LeaveBuffer => {
                        if let Some((_, _, state)) = self.get_focused_mut()
                            && let Some(buffer) =
                                state.buffer.upstream().cloned()
                        {
                            return self.leave_buffer(clients, config, buffer);
                        }
                    }
                    ToggleNicklist => {
                        if let Some((_, _, pane)) = self.get_focused_mut() {
                            if let Some(buffer) = pane.buffer.data() {
                                let settings = self.buffer_settings.entry(
                                    &buffer,
                                    Some(config.buffer.clone().into()),
                                );
                                settings.channel.nicklist.toggle_visibility();
                            }

                            self.last_changed = Some(Instant::now());
                            return (Task::none(), None);
                        }
                    }
                    ToggleTopic => {
                        if let Some((_, _, pane)) = self.get_focused_mut() {
                            if let Some(buffer) = pane.buffer.data() {
                                let settings = self.buffer_settings.entry(
                                    &buffer,
                                    Some(config.buffer.clone().into()),
                                );
                                settings
                                    .channel
                                    .topic_banner
                                    .toggle_visibility();
                            }

                            self.last_changed = Some(Instant::now());
                            return (Task::none(), None);
                        }
                    }
                    ToggleSidebar => {
                        self.side_menu.toggle_visibility();
                    }
                    CommandBar => {
                        return (
                            self.toggle_command_bar(
                                &closed_buffers(self, clients),
                                version,
                                config,
                                theme,
                            ),
                            None,
                        );
                    }
                    ReloadConfiguration => {
                        return (
                            Task::perform(
                                Config::load(),
                                Message::ConfigReloaded,
                            ),
                            None,
                        );
                    }
                    FileTransfers => {
                        return (
                            self.toggle_internal_buffer(
                                clients,
                                config,
                                buffer::Internal::FileTransfers,
                            ),
                            None,
                        );
                    }
                    Logs => {
                        return (
                            self.toggle_internal_buffer(
                                clients,
                                config,
                                buffer::Internal::Logs,
                            ),
                            None,
                        );
                    }
                    ThemeEditor => {
                        return (
                            self.toggle_theme_editor(
                                theme,
                                main_window,
                                config,
                            ),
                            None,
                        );
                    }
                    Highlights => {
                        return (
                            self.toggle_internal_buffer(
                                clients,
                                config,
                                buffer::Internal::Highlights,
                            ),
                            None,
                        );
                    }
                    ToggleFullscreen => {
                        return (window::toggle_fullscreen(), None);
                    }
                    QuitApplication => {
                        return (self.exit(clients, config), None);
                    }
                    ScrollUpPage => {
                        return (
                            self.get_focused_mut().map_or_else(
                                Task::none,
                                |(window, pane, state)| {
                                    state.buffer.scroll_up_page().map(
                                        move |message| {
                                            Message::Pane(
                                                window,
                                                pane::Message::Buffer(
                                                    pane, message,
                                                ),
                                            )
                                        },
                                    )
                                },
                            ),
                            None,
                        );
                    }
                    ScrollDownPage => {
                        return (
                            self.get_focused_mut().map_or_else(
                                Task::none,
                                |(window, pane, state)| {
                                    state.buffer.scroll_down_page().map(
                                        move |message| {
                                            Message::Pane(
                                                window,
                                                pane::Message::Buffer(
                                                    pane, message,
                                                ),
                                            )
                                        },
                                    )
                                },
                            ),
                            None,
                        );
                    }
                    ScrollToTop => {
                        if config.buffer.chathistory.infinite_scroll
                            && let Some((_, _, state)) = self.get_focused()
                            && let Some(buffer) = state.buffer.data()
                        {
                            self.request_older_chathistory(clients, &buffer);
                        }

                        return (
                            self.get_focused_mut().map_or_else(
                                Task::none,
                                |(window, id, pane)| {
                                    pane.buffer.scroll_to_start(config).map(
                                        move |message| {
                                            Message::Pane(
                                                window,
                                                pane::Message::Buffer(
                                                    id, message,
                                                ),
                                            )
                                        },
                                    )
                                },
                            ),
                            None,
                        );
                    }
                    ScrollToBottom => {
                        let task = self.get_focused_mut().map_or_else(
                            Task::none,
                            |(window, pane, state)| {
                                let mut task = state
                                    .buffer
                                    .scroll_to_end(config)
                                    .map(move |message| {
                                        Message::Pane(
                                            window,
                                            pane::Message::Buffer(
                                                pane, message,
                                            ),
                                        )
                                    });

                                if config
                                    .buffer
                                    .mark_as_read
                                    .on_scroll_to_bottom
                                {
                                    task =
                                        task.chain(Task::done(Message::Pane(
                                            window,
                                            pane::Message::MarkAsRead,
                                        )));
                                }

                                task
                            },
                        );

                        return (task, None);
                    }
                    CycleNextUnreadBuffer => {
                        let all_buffers =
                            all_buffers_with_has_unread(clients, &self.history);
                        let open_buffers = open_buffers(self);

                        if let Some((window, pane, state, history)) =
                            self.get_focused_with_history_mut()
                            && let Some(buffer) = cycle_next_unread_buffer(
                                state.buffer.upstream(),
                                all_buffers,
                                &open_buffers,
                            )
                        {
                            mark_as_read_on_buffer_close(
                                &state.buffer,
                                history,
                                clients,
                                config,
                            );

                            state.buffer = Buffer::from_data(
                                data::Buffer::Upstream(buffer),
                                state.size,
                                config,
                            );
                            self.last_changed = Some(Instant::now());
                            return (self.focus_pane(window, pane), None);
                        }
                    }
                    CyclePreviousUnreadBuffer => {
                        let all_buffers =
                            all_buffers_with_has_unread(clients, &self.history);
                        let open_buffers = open_buffers(self);

                        if let Some((window, pane, state, history)) =
                            self.get_focused_with_history_mut()
                            && let Some(buffer) = cycle_previous_unread_buffer(
                                state.buffer.upstream(),
                                all_buffers,
                                &open_buffers,
                            )
                        {
                            mark_as_read_on_buffer_close(
                                &state.buffer,
                                history,
                                clients,
                                config,
                            );

                            state.buffer = Buffer::from_data(
                                data::Buffer::Upstream(buffer),
                                state.size,
                                config,
                            );
                            self.last_changed = Some(Instant::now());
                            return (self.focus_pane(window, pane), None);
                        }
                    }
                    MarkAsRead => {
                        if let Some((_, _, pane)) = self.get_focused_mut()
                            && let Some(kind) = pane
                                .buffer
                                .data()
                                .and_then(history::Kind::from_buffer)
                        {
                            mark_as_read(
                                kind,
                                &mut self.history,
                                clients,
                                TokenPriority::User,
                            );
                        }
                    }
                }
            }
            Message::FileTransfer(update) => {
                self.file_transfers.update(update, config);
            }
            Message::SendFileSelected(server, to, path) => {
                if let Some(server_handle) = clients.get_server_handle(&server)
                {
                    let casemapping = clients.get_casemapping(&server);

                    let query = target::Query::from(&to);

                    if let Some(path) = path
                        && let Some(event) = self.file_transfers.send(
                            file_transfer::SendRequest {
                                to,
                                path,
                                server: server.clone(),
                                server_handle: server_handle.clone(),
                            },
                            config,
                        )
                    {
                        return (
                            self.handle_file_transfer_event(
                                &server,
                                casemapping,
                                &query,
                                event,
                                &config.buffer,
                            ),
                            None,
                        );
                    }
                }
            }
            Message::CloseContextMenu(window, any_closed) => {
                if !any_closed {
                    if let Some((_, _, state)) = self.get_focused_mut()
                        && state.buffer.close_picker()
                    {
                        return (Task::none(), None);
                    }

                    if self.is_pane_maximized() && window == self.main_window()
                    {
                        self.panes.main.restore();
                    }
                }
            }
            Message::ThemeEditor(message) => {
                let mut editor_event = None;
                let mut event = None;
                let mut tasks = vec![];

                if let Some(editor) = self.theme_editor.as_mut() {
                    let (task, event) = editor.update(message, theme);

                    tasks.push(task.map(Message::ThemeEditor));
                    editor_event = event;
                }

                if let Some(editor_event) = editor_event {
                    match editor_event {
                        theme_editor::Event::Close => {
                            if let Some(editor) = self.theme_editor.take() {
                                tasks.push(window::close(editor.window));
                            }
                        }
                        theme_editor::Event::ReloadThemes => {
                            event = Some(Event::ReloadThemes);
                        }
                    }
                }

                return (Task::batch(tasks), event);
            }
            Message::ConfigReloaded(config_result) => {
                return (
                    Task::none(),
                    Some(Event::ConfigReloaded(config_result)),
                );
            }
            Message::Client(message) => match message {
                client::Message::ChatHistoryRequest(server, subcommand) => {
                    clients.send_chathistory_request(
                        &server,
                        subcommand,
                        TokenPriority::High,
                    );
                }
                client::Message::ChatHistoryTargetsTimestampUpdated(
                    server,
                    timestamp,
                    Ok(()),
                ) => {
                    log::debug!(
                        "updated targets timestamp for {server} to {timestamp}"
                    );
                }
                client::Message::ChatHistoryTargetsTimestampUpdated(
                    server,
                    timestamp,
                    Err(error),
                ) => {
                    log::warn!(
                        "failed to update targets timestamp for {server} to {timestamp}: {error}"
                    );
                }
                client::Message::RequestNewerChatHistory(
                    server,
                    target,
                    server_time,
                ) => {
                    let message_reference_types = clients
                        .get_server_chathistory_message_reference_types(
                            &server,
                        );

                    let message_reference = self
                        .history
                        .last_can_reference_before(
                            server.clone(),
                            target.clone(),
                            server_time,
                        )
                        .map_or(MessageReference::None, |message_references| {
                            message_references
                                .message_reference(&message_reference_types)
                        });

                    let limit = clients.get_server_chathistory_limit(&server);

                    clients.send_chathistory_request(
                        &server,
                        ChatHistorySubcommand::Latest(
                            target,
                            message_reference,
                            limit,
                        ),
                        TokenPriority::High,
                    );
                }
                client::Message::RequestChatHistoryTargets(
                    server,
                    timestamp,
                    server_time,
                ) => {
                    let start_message_reference = timestamp
                        .map_or(MessageReference::None, |timestamp| {
                            MessageReference::Timestamp(timestamp)
                        });

                    let end_message_reference =
                        MessageReference::Timestamp(server_time);

                    let limit = clients.get_server_chathistory_limit(&server);

                    clients.send_chathistory_request(
                        &server,
                        ChatHistorySubcommand::Targets(
                            start_message_reference,
                            end_message_reference,
                            limit,
                        ),
                        TokenPriority::High,
                    );
                }
            },
            Message::LoadPreview((url, Ok(preview))) => {
                log::debug!("Preview loaded for {url}");
                if let hash_map::Entry::Occupied(mut entry) =
                    self.previews.entry(url)
                {
                    *entry.get_mut() = preview::State::Loaded(preview);
                }
            }
            Message::LoadPreview((url, Err(error))) => {
                log::info!("Failed to load preview for {url}: {error}");
                if self.previews.contains_key(&url) {
                    self.previews.insert(url, preview::State::Error(error));
                }
            }
            Message::NewWindow(window, pane) => {
                let (state, pane) = pane_grid::State::new(pane);
                self.panes.popout.insert(window, state);

                return (self.focus_pane(window, pane), None);
            }
        }

        (Task::none(), None)
    }

    pub fn view_window<'a>(
        &'a self,
        window: window::Id,
        clients: &'a client::Map,
        config: &'a Config,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        if let Some(state) = self.panes.popout.get(&window) {
            let content = container(
                PaneGrid::new(state, |id, pane, _maximized| {
                    let is_focused = self.focus == Focus { window, pane: id };
                    let buffer = pane.buffer.data();
                    let settings = buffer
                        .as_ref()
                        .and_then(|b| self.buffer_settings.get(b));

                    pane.view(
                        id,
                        1,
                        is_focused,
                        false,
                        clients,
                        &self.file_transfers,
                        &self.history,
                        &self.previews,
                        &self.side_menu,
                        config,
                        theme,
                        settings,
                        window != self.main_window(),
                    )
                })
                .spacing(4)
                .on_click(pane::Message::PaneClicked),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8);

            return Element::new(content)
                .map(move |message| Message::Pane(window, message));
        } else if let Some(editor) = self.theme_editor.as_ref()
            && editor.window == window
        {
            return editor.view(theme).map(Message::ThemeEditor);
        }

        column![].into()
    }

    pub fn view<'a>(
        &'a self,
        servers: &'a server::Map,
        clients: &'a client::Map,
        version: &'a Version,
        config: &'a Config,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let pane_grid: Element<_> =
            PaneGrid::new(&self.panes.main, |id, pane, maximized| {
                let is_focused = self.focus
                    == Focus {
                        window: self.main_window(),
                        pane: id,
                    };
                let panes = self.panes.main.panes.len();
                let buffer = pane.buffer.data();
                let settings =
                    buffer.as_ref().and_then(|b| self.buffer_settings.get(b));

                pane.view(
                    id,
                    panes,
                    is_focused,
                    maximized,
                    clients,
                    &self.file_transfers,
                    &self.history,
                    &self.previews,
                    &self.side_menu,
                    config,
                    theme,
                    settings,
                    false,
                )
            })
            .on_click(pane::Message::PaneClicked)
            .on_resize(6, pane::Message::PaneResized)
            .on_drag(pane::Message::PaneDragged)
            .spacing(4)
            .into();

        let pane_grid =
            container(pane_grid.map(move |message| {
                Message::Pane(self.main_window(), message)
            }))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8);

        let side_menu = self
            .side_menu
            .view(
                servers,
                clients,
                &self.history,
                &self.panes,
                self.focus,
                config,
                &self.file_transfers,
                version,
                theme,
            )
            .map(|e| e.map(Message::Sidebar));

        let content = match config.sidebar.position {
            data::config::sidebar::Position::Left
            | data::config::sidebar::Position::Top => {
                vec![
                    side_menu.unwrap_or_else(|| row![].into()),
                    pane_grid.into(),
                ]
            }
            data::config::sidebar::Position::Right
            | data::config::sidebar::Position::Bottom => {
                vec![
                    pane_grid.into(),
                    side_menu.unwrap_or_else(|| row![].into()),
                ]
            }
        };

        let base: Element<Message> = if config.sidebar.position.is_horizontal()
        {
            Column::with_children(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            Row::with_children(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        let base = if let Some(command_bar) = self.command_bar.as_ref() {
            let background = anchored_overlay(
                base,
                container(
                    Space::new().width(Length::Fill).height(Length::Fill),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::container::transparent_overlay),
                anchored_overlay::Anchor::BelowTopCentered,
                0.0,
            );

            // Task bar
            anchored_overlay(
                background,
                command_bar
                    .view(
                        &all_buffers(clients, &self.history),
                        self.focus,
                        self.buffer_resize_action(),
                        version,
                        config,
                        self.main_window(),
                    )
                    .map(Message::Task),
                anchored_overlay::Anchor::BelowTopCentered,
                10.0,
            )
        } else {
            // Align `base` into same view tree shape
            // as `anchored_overlay` to prevent diff
            // from firing when displaying command bar
            column![column![base]].into()
        };

        shortcut(base, config.keyboard.shortcuts(), Message::Shortcut)
    }

    pub fn handle_buffer_event(
        &mut self,
        window: window::Id,
        id: pane_grid::Pane,
        event: buffer::Event,
        clients: &mut data::client::Map,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        let Some(pane) = self.panes.get_mut(window, id) else {
            return (Task::none(), None);
        };

        match event {
            buffer::Event::ContextMenu(event) => {
                let mut tasks =
                    vec![context_menu::close(convert::identity).map(
                        move |any_closed| {
                            Message::CloseContextMenu(window, any_closed)
                        },
                    )];

                match event {
                    buffer::context_menu::Event::CopyUrl(url) => {
                        tasks.push(clipboard::write(url));
                    }
                    buffer::context_menu::Event::ToggleAccessLevel(
                        server,
                        channel,
                        nick,
                        mode,
                    ) => {
                        let buffer = buffer::Upstream::Channel(
                            server.clone(),
                            channel.clone(),
                        );

                        let command = command::Irc::Mode(
                            channel.to_string(),
                            Some(mode),
                            Some(vec![nick.to_string()]),
                        );
                        let input = data::Input::command(buffer, command);

                        if let Some(encoded) = input.encoded() {
                            clients.send(
                                &input.buffer,
                                encoded,
                                TokenPriority::User,
                            );
                        }
                    }
                    buffer::context_menu::Event::SendWhois(server, nick) => {
                        let buffer =
                            pane.buffer.upstream().cloned().unwrap_or_else(
                                || buffer::Upstream::Server(server.clone()),
                            );

                        let command =
                            command::Irc::Whois(None, nick.to_string());

                        let input =
                            data::Input::command(buffer.clone(), command);

                        if let Some(encoded) = input.encoded() {
                            clients.send(
                                &input.buffer,
                                encoded,
                                TokenPriority::User,
                            );
                        }

                        if let Some(nick) = clients.nickname(buffer.server()) {
                            let mut user = nick.to_owned().into();
                            let mut channel_users = None;
                            let chantypes =
                                clients.get_chantypes(buffer.server());
                            let statusmsg =
                                clients.get_statusmsg(buffer.server());
                            let casemapping =
                                clients.get_casemapping(buffer.server());
                            let supports_echoes = clients
                                .get_server_supports_echoes(buffer.server());

                            // Resolve our attributes if sending this message in a channel
                            if let buffer::Upstream::Channel(server, channel) =
                                &buffer
                            {
                                channel_users =
                                    clients.get_channel_users(server, channel);

                                if let Some(user_with_attributes) = clients
                                    .resolve_user_attributes(
                                        server, channel, &user,
                                    )
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
                                config,
                            ) {
                                for message in messages {
                                    if let Some(task) =
                                        self.history.record_message(
                                            input.server(),
                                            casemapping,
                                            message,
                                            &config.buffer,
                                        )
                                    {
                                        tasks.push(Task::perform(
                                            task,
                                            Message::History,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    buffer::context_menu::Event::OpenQuery(
                        server,
                        query,
                        buffer_action,
                    ) => {
                        let buffer = buffer::Upstream::Query(server, query);

                        tasks.push(self.open_buffer(
                            data::Buffer::Upstream(buffer),
                            buffer_action,
                            clients,
                            config,
                        ));
                    }
                    buffer::context_menu::Event::InsertNickname(nick) => {
                        if let Some((_, _, pane, history)) =
                            self.get_focused_with_history_mut()
                        {
                            tasks.push(
                                pane.buffer
                                    .insert_user_to_input(
                                        nick,
                                        history,
                                        &config.buffer.text_input.autocomplete,
                                    )
                                    .map(move |message| {
                                        Message::Pane(
                                            window,
                                            pane::Message::Buffer(id, message),
                                        )
                                    }),
                            );
                        }
                    }
                    buffer::context_menu::Event::SendFile(server, nick) => {
                        tasks.push(Task::perform(
                            async move {
                                rfd::AsyncFileDialog::new()
                                    .pick_file()
                                    .await
                                    .map(|handle| handle.path().to_path_buf())
                            },
                            move |file| {
                                Message::SendFileSelected(
                                    server.clone(),
                                    nick.clone(),
                                    file,
                                )
                            },
                        ));
                    }
                    buffer::context_menu::Event::CtcpRequest(
                        command,
                        server,
                        nick,
                        params,
                    ) => {
                        let buffer =
                            pane.buffer.upstream().cloned().unwrap_or_else(
                                || buffer::Upstream::Server(server.clone()),
                            );

                        let command = command::Irc::Ctcp(
                            command,
                            nick.to_string(),
                            params,
                        );

                        let input =
                            data::Input::command(buffer.clone(), command);

                        if let Some(encoded) = input.encoded() {
                            clients.send(
                                &input.buffer,
                                encoded,
                                TokenPriority::High,
                            );
                        }
                    }
                    buffer::context_menu::Event::CopyTimestamp(
                        date_time,
                        format,
                    ) => {
                        if let Some(format) = format {
                            tasks.push(clipboard::write(
                                date_time
                                    .with_timezone(&Local)
                                    .format(&format)
                                    .to_string(),
                            ));
                        } else {
                            tasks.push(clipboard::write(
                                date_time.to_rfc3339_opts(
                                    SecondsFormat::Millis,
                                    true,
                                ),
                            ));
                        }
                    }
                    buffer::context_menu::Event::DeleteMessage(
                        server_time,
                        hash,
                    ) => {
                        if let Some(kind) =
                            pane.buffer.upstream().map(|buffer| {
                                history::Kind::from_input_buffer(buffer.clone())
                            })
                            && let Some(future) = self.history.remove_message(
                                kind,
                                server_time,
                                hash,
                                false,
                            )
                        {
                            tasks.push(
                                Task::future(future).map(Message::History),
                            );
                        }
                    }
                    buffer::context_menu::Event::ResendMessage(
                        server_time,
                        hash,
                    ) => {
                        if let Some(kind) =
                            pane.buffer.upstream().map(|buffer| {
                                history::Kind::from_input_buffer(buffer.clone())
                            })
                            && let Some(future) = self.history.remove_message(
                                kind,
                                server_time,
                                hash,
                                true,
                            )
                        {
                            tasks.push(
                                Task::future(future).map(Message::History),
                            );
                        }
                    }
                }

                return (Task::batch(tasks), None);
            }
            buffer::Event::OpenBuffers(targets) => {
                let mut tasks = vec![];

                if let Some(server) = pane
                    .buffer
                    .upstream()
                    .map(buffer::Upstream::server)
                    .cloned()
                {
                    for (target, buffer_action) in targets {
                        tasks.push(self.open_target(
                            server.clone(),
                            target,
                            clients,
                            buffer_action,
                            config,
                        ));
                    }
                }

                return (Task::batch(tasks), None);
            }
            buffer::Event::LeaveBuffers(targets, reason) => {
                if let Some(server) = pane
                    .buffer
                    .upstream()
                    .map(buffer::Upstream::server)
                    .cloned()
                {
                    let mut tasks = vec![];

                    for target in targets {
                        tasks.push(self.leave_server_target(
                            clients,
                            config,
                            server.clone(),
                            target,
                            reason.clone(),
                        ));
                    }

                    return (Task::batch(tasks), None);
                }
            }
            buffer::Event::History(history_task) => {
                return (history_task.map(Message::History), None);
            }
            buffer::Event::GoToMessage(server, channel, message) => {
                let buffer = data::Buffer::Upstream(buffer::Upstream::Channel(
                    server, channel,
                ));

                let mut tasks = vec![];

                if self.panes.get_mut_by_buffer(&buffer).is_none() {
                    tasks.push(self.open_buffer(
                        buffer.clone(),
                        config.actions.buffer.click_highlight,
                        clients,
                        config,
                    ));
                }

                if let Some((window, pane, state)) =
                    self.panes.get_mut_by_buffer(&buffer)
                {
                    tasks.push(
                        state
                            .buffer
                            .scroll_to_message(message, &self.history, config)
                            .map(move |message| {
                                Message::Pane(
                                    window,
                                    pane::Message::Buffer(pane, message),
                                )
                            }),
                    );
                }

                return (Task::batch(tasks), None);
            }
            buffer::Event::RequestOlderChatHistory => {
                if let Some(buffer) = pane.buffer.data() {
                    self.request_older_chathistory(clients, &buffer);
                }
            }
            buffer::Event::PreviewChanged => {
                let visible = self.panes.visible_urls();
                let tracking =
                    self.previews.keys().cloned().collect::<HashSet<_>>();
                let missing =
                    visible.difference(&tracking).cloned().collect::<Vec<_>>();
                let removed = tracking.difference(&visible);

                for url in &missing {
                    self.previews.insert(url.clone(), preview::State::Loading);
                }

                for url in removed {
                    self.previews.remove(url);
                }

                return (
                    Task::batch(missing.into_iter().map(|url| {
                        Task::perform(
                            data::preview::load(
                                url.clone(),
                                config.preview.clone(),
                            ),
                            move |result| {
                                Message::LoadPreview((url.clone(), result))
                            },
                        )
                    })),
                    None,
                );
            }
            buffer::Event::HidePreview(kind, hash, url) => {
                self.history.hide_preview(kind, hash, url);
            }
            buffer::Event::MarkAsRead(kind) => {
                mark_as_read(
                    kind,
                    &mut self.history,
                    clients,
                    TokenPriority::User,
                );
            }
            buffer::Event::OpenUrl(url) => {
                return (
                    Task::none(),
                    Some(Event::OpenUrl(
                        url,
                        config.buffer.url.prompt_before_open,
                    )),
                );
            }
            buffer::Event::ImagePreview(path, url) => {
                return (Task::none(), Some(Event::ImagePreview(path, url)));
            }
        }

        (Task::none(), None)
    }

    pub fn handle_event(
        &mut self,
        window: window::Id,
        event: event::Event,
        clients: &mut data::client::Map,
        version: &Version,
        config: &Config,
        theme: &mut Theme,
    ) -> Task<Message> {
        use event::Event::*;

        match event {
            Escape => {
                // Order of operations
                //
                // - Close command bar (if main window)
                // - Close context menu
                // - Close command/emoji picker
                // - Restore maximized pane (if main window)
                if self.command_bar.is_some() && window == self.main_window() {
                    self.toggle_command_bar(
                        &closed_buffers(self, clients),
                        version,
                        config,
                        theme,
                    )
                } else {
                    context_menu::close(convert::identity).map(
                        move |any_closed| {
                            Message::CloseContextMenu(window, any_closed)
                        },
                    )
                }
            }
            Copy => selectable_text::selected(|selected_text| {
                Message::SelectedText(
                    selected_text,
                    advanced::clipboard::Kind::Standard,
                )
            }),
            LeftClick => self.refocus_pane(),
            UpdatePrimaryClipboard => {
                selectable_text::selected(|selected_text| {
                    Message::SelectedText(
                        selected_text,
                        advanced::clipboard::Kind::Primary,
                    )
                })
            }
        }
    }

    fn toggle_theme_editor(
        &mut self,
        theme: &mut Theme,
        main_window: &Window,
        config: &Config,
    ) -> Task<Message> {
        if let Some(editor) = self.theme_editor.take() {
            *theme = theme.selected();
            window::close(editor.window)
        } else {
            let (editor, task) = ThemeEditor::open(main_window, config);

            self.theme_editor = Some(editor);

            task.then(|_| Task::none())
        }
    }

    fn toggle_internal_buffer(
        &mut self,
        clients: &mut data::client::Map,
        config: &Config,
        buffer: buffer::Internal,
    ) -> Task<Message> {
        let panes = self.panes.clone();

        let open = panes.iter().find_map(|(window_id, pane, state)| {
            (state.buffer.internal().as_ref() == Some(&buffer))
                .then_some((window_id, pane))
        });

        if let Some((window, pane)) = open {
            self.close_pane(clients, config, window, pane)
        } else {
            self.open_buffer(
                data::Buffer::Internal(buffer),
                config.actions.buffer.local,
                clients,
                config,
            )
        }
    }

    fn open_buffer(
        &mut self,
        buffer: data::Buffer,
        buffer_action: BufferAction,
        clients: &mut data::client::Map,
        config: &Config,
    ) -> Task<Message> {
        let panes = self.panes.clone();

        self.last_changed = Some(Instant::now());

        match buffer_action {
            BufferAction::ReplacePane => {
                // If buffer already is open, we swap it with focused pane.
                for (window, id, pane) in panes.iter() {
                    if pane.buffer.data().as_ref() == Some(&buffer) {
                        if window != self.focus.window || id != self.focus.pane
                        {
                            return self.swap_pane_with_focus(window, id);
                        } else {
                            return Task::none();
                        }
                    }
                }

                let Focus { window, pane } = self.focus;

                if let Some(state) = self.panes.get_mut(window, pane) {
                    mark_as_read_on_buffer_close(
                        &state.buffer,
                        &mut self.history,
                        clients,
                        config,
                    );

                    state.buffer =
                        Buffer::from_data(buffer, state.size, config);
                    self.last_changed = Some(Instant::now());

                    Task::batch(vec![
                        self.reset_pane(window, pane),
                        self.focus_pane(window, pane),
                    ])
                } else {
                    log::error!("Didn't find any panes to replace");
                    Task::none()
                }
            }
            BufferAction::NewPane => {
                // If buffer already is open, we focus it.
                for (window, id, pane) in panes.iter() {
                    if pane.buffer.data().as_ref() == Some(&buffer) {
                        self.focus = Focus { window, pane: id };

                        return self.focus_pane(window, id);
                    }
                }

                // If we only have one pane, and its empty, we replace it.
                if self.panes.len() == 1 {
                    for (id, pane) in panes.main.iter() {
                        if matches!(pane.buffer, Buffer::Empty) {
                            self.panes.main.panes.entry(*id).and_modify(|p| {
                                *p = Pane::new(Buffer::from_data(
                                    buffer, p.size, config,
                                ));
                            });
                            self.last_changed = Some(Instant::now());

                            return self.focus_pane(self.main_window(), *id);
                        }
                    }
                }

                let (pane_to_split, pane_to_split_state) = {
                    if matches!(
                        config.pane.split_axis,
                        config::pane::SplitAxis::LargestShorter
                    ) && let Some((pane, pane_state)) =
                        self.panes.main.panes.iter().reduce(
                            |(acc_pane, acc_pane_state), (pane, pane_state)| {
                                let pane_area = pane_state.size.width
                                    * pane_state.size.height;
                                let acc_pane_area = acc_pane_state.size.width
                                    * acc_pane_state.size.height;

                                if pane_area > acc_pane_area {
                                    (pane, pane_state)
                                } else {
                                    (acc_pane, acc_pane_state)
                                }
                            },
                        )
                    {
                        (*pane, pane_state)
                    } else if self.focus.window == self.main_window()
                        && let Some(pane_state) =
                            self.panes.main.panes.get(&self.focus.pane)
                    {
                        (self.focus.pane, pane_state)
                    } else if let Some((pane, pane_state)) =
                        self.panes.main.panes.iter().last()
                    {
                        (*pane, pane_state)
                    } else {
                        log::error!("Didn't find any panes to split");
                        return Task::none();
                    }
                };

                let split_axis = match config.pane.split_axis {
                    config::pane::SplitAxis::Horizontal => {
                        pane_grid::Axis::Horizontal
                    }
                    config::pane::SplitAxis::Vertical => {
                        pane_grid::Axis::Vertical
                    }
                    config::pane::SplitAxis::Shorter
                    | config::pane::SplitAxis::LargestShorter => {
                        if pane_to_split_state.size.height
                            < pane_to_split_state.size.width
                        {
                            pane_grid::Axis::Vertical
                        } else {
                            pane_grid::Axis::Horizontal
                        }
                    }
                };

                let result = self.panes.main.split(
                    split_axis,
                    pane_to_split,
                    Pane::new(Buffer::from_data(
                        buffer,
                        pane_to_split_state.size,
                        config,
                    )),
                );

                if let Some((pane, _)) = result {
                    return self.focus_pane(self.main_window(), pane);
                }

                Task::none()
            }
            BufferAction::NewWindow => {
                iced::window::position(self.main_window()).then({
                    let pane = Pane::new(Buffer::from_data(
                        buffer.clone(),
                        Size::default(),
                        config,
                    ));

                    let config = config.clone();
                    move |main_window_position| {
                        let (_, task) = window::open(window::Settings {
                            // Just big enough to show all components in combobox
                            position: main_window_position
                                .map(|point| {
                                    window::Position::Specific(
                                        point + Vector::new(20.0, 20.0),
                                    )
                                })
                                .unwrap_or_default(),
                            exit_on_close_request: false,
                            ..window::settings(&config)
                        });

                        task.map({
                            let pane = pane.clone();
                            move |id| Message::NewWindow(id, pane.clone())
                        })
                    }
                })
            }
        }
    }

    pub fn leave_all_queries(
        &mut self,
        clients: &mut data::client::Map,
        config: &Config,
        server: Server,
        queries: Vec<target::Query>,
    ) -> Task<Message> {
        let tasks: Vec<Task<Message>> = queries
            .into_iter()
            .map(|query| {
                let buffer =
                    buffer::Upstream::Query(server.clone(), query.clone());

                self.leave_buffer(clients, config, buffer).0
            })
            .collect();


        Task::batch(tasks)
    }

    pub fn leave_buffer(
        &mut self,
        clients: &mut data::client::Map,
        config: &Config,
        buffer: buffer::Upstream,
    ) -> (Task<Message>, Option<Event>) {
        let open = self.panes.iter().find_map(|(window, pane, state)| {
            (state.buffer.upstream() == Some(&buffer)).then_some((window, pane))
        });

        let mut tasks = vec![];

        // Close pane
        if let Some((window, pane)) = open {
            tasks.push(self.close_pane(clients, config, window, pane));
        }

        match buffer.clone() {
            buffer::Upstream::Server(server) => {
                (Task::batch(tasks), Some(Event::QuitServer(server, None)))
            }
            buffer::Upstream::Channel(server, channel) => {
                // Send part & close history file
                let command = command::Irc::Part(channel.to_string(), None);
                let input = data::Input::command(buffer.clone(), command);

                if let Some(encoded) = input.encoded() {
                    clients.send(&buffer, encoded, TokenPriority::High);
                }

                tasks.push(
                    self.history
                        .close(history::Kind::Channel(server, channel), clients)
                        .map_or_else(Task::none, |task| {
                            Task::perform(task, Message::History)
                        }),
                );

                (Task::batch(tasks), None)
            }
            buffer::Upstream::Query(server, nick) => {
                tasks.push(
                    self.history
                        .close(history::Kind::Query(server, nick), clients)
                        .map_or_else(Task::none, |task| {
                            Task::perform(task, Message::History)
                        }),
                );

                // No PART to send, just close history
                (Task::batch(tasks), None)
            }
        }
    }

    pub fn leave_server_target(
        &mut self,
        clients: &mut data::client::Map,
        config: &Config,
        server: Server,
        target: Target,
        reason: Option<String>,
    ) -> Task<Message> {
        let open = self.panes.iter().find_map(|(window, pane, state)| {
            (state.buffer.server() == Some(server.clone())
                && state.buffer.target() == Some(target.clone()))
            .then_some((window, pane))
        });

        let mut tasks = vec![];

        // Close pane
        if let Some((window, pane)) = open {
            tasks.push(self.close_pane(clients, config, window, pane));
        }

        match target {
            Target::Channel(channel) => {
                let buffer = data::buffer::Upstream::Channel(
                    server.clone(),
                    channel.clone(),
                );

                // Send part & close history file
                let command = command::Irc::Part(channel.to_string(), reason);
                let input = data::Input::command(buffer.clone(), command);

                if let Some(encoded) = input.encoded() {
                    clients.send(&buffer, encoded, TokenPriority::User);
                }

                tasks.push(
                    self.history
                        .close(history::Kind::Channel(server, channel), clients)
                        .map_or_else(Task::none, |task| {
                            Task::perform(task, Message::History)
                        }),
                );

                Task::batch(tasks)
            }
            Target::Query(nick) => {
                tasks.push(
                    self.history
                        .close(history::Kind::Query(server, nick), clients)
                        .map_or_else(Task::none, |task| {
                            Task::perform(task, Message::History)
                        }),
                );

                // No PART to send, just close history
                Task::batch(tasks)
            }
        }
    }

    pub fn record_message(
        &mut self,
        server: &Server,
        casemapping: isupport::CaseMap,
        message: data::Message,
        buffer_config: &config::Buffer,
    ) -> Task<Message> {
        if let Some(task) = self.history.record_message(
            server,
            casemapping,
            message,
            buffer_config,
        ) {
            Task::perform(task, Message::History)
        } else {
            Task::none()
        }
    }

    pub fn record_log(&mut self, record: data::log::Record) -> Task<Message> {
        if let Some(task) = self.history.record_log(record) {
            Task::perform(task, Message::History)
        } else {
            Task::none()
        }
    }

    pub fn record_highlight(
        &mut self,
        message: data::Message,
    ) -> Task<Message> {
        self.history
            .record_highlight(message)
            .map_or_else(Task::none, |task| {
                Task::perform(task, Message::History)
            })
    }

    pub fn get_oldest_message_reference(
        &self,
        server: &Server,
        target: Target,
        message_reference_types: &[isupport::MessageReferenceType],
    ) -> MessageReference {
        if let Some(first_can_reference) = self
            .history
            .first_can_reference(server.clone(), target.clone())
        {
            for message_reference_type in message_reference_types {
                match message_reference_type {
                    isupport::MessageReferenceType::MessageId => {
                        if let Some(id) = &first_can_reference.id {
                            return MessageReference::MessageId(id.clone());
                        }
                    }
                    isupport::MessageReferenceType::Timestamp => {
                        return MessageReference::Timestamp(
                            first_can_reference.server_time,
                        );
                    }
                }
            }
        }

        MessageReference::None
    }

    pub fn request_older_chathistory(
        &self,
        clients: &mut data::client::Map,
        buffer: &data::Buffer,
    ) {
        let Some(upstream) = buffer.upstream() else {
            return;
        };

        let server = upstream.server();

        if clients.get_server_supports_chathistory(server)
            && let Some(target) = upstream.target()
        {
            if clients.get_chathistory_exhausted(server, &target) {
                return;
            }

            let message_reference_types =
                clients.get_server_chathistory_message_reference_types(server);

            let first_can_reference = self.get_oldest_message_reference(
                server,
                target.clone(),
                &message_reference_types,
            );

            let subcommand =
                if matches!(first_can_reference, MessageReference::None) {
                    ChatHistorySubcommand::Latest(
                        target,
                        first_can_reference,
                        clients.get_server_chathistory_limit(server),
                    )
                } else {
                    ChatHistorySubcommand::Before(
                        target,
                        first_can_reference,
                        clients.get_server_chathistory_limit(server),
                    )
                };

            clients.send_chathistory_request(
                server,
                subcommand,
                TokenPriority::User,
            );
        }
    }

    pub fn broadcast(
        &mut self,
        server: &Server,
        casemapping: isupport::CaseMap,
        config: &Config,
        sent_time: DateTime<Utc>,
        broadcast: Broadcast,
    ) -> Task<Message> {
        Task::batch(
            self.history
                .broadcast(server, casemapping, broadcast, config, sent_time)
                .into_iter()
                .map(|task| Task::perform(task, Message::History)),
        )
    }

    pub fn block_message(
        &self,
        message: &mut data::Message,
        kind: &history::Kind,
        casemapping: isupport::CaseMap,
        buffer_config: &config::Buffer,
    ) {
        self.history
            .block_message(message, kind, casemapping, buffer_config);
    }

    pub fn update_read_marker(
        &mut self,
        kind: impl Into<history::Kind> + 'static,
        read_marker: ReadMarker,
    ) -> Task<Message> {
        if let Some(task) = self.history.update_read_marker(kind, read_marker) {
            Task::perform(task, Message::History)
        } else {
            Task::none()
        }
    }

    pub fn load_metadata(
        &mut self,
        clients: &data::client::Map,
        server: Server,
        target: Target,
        server_time: DateTime<Utc>,
    ) -> Task<Message> {
        let command = self
            .history
            .load_metadata(server.clone(), target.clone())
            .map_or(Task::none(), |task| Task::perform(task, Message::History));

        if clients.get_server_supports_chathistory(&server) {
            command.chain(Task::done(Message::Client(
                data::client::Message::RequestNewerChatHistory(
                    server,
                    target,
                    server_time,
                ),
            )))
        } else {
            command
        }
    }

    pub fn load_chathistory_targets_timestamp(
        &self,
        clients: &data::client::Map,
        server: &Server,
        server_time: DateTime<Utc>,
    ) -> Option<Task<Message>> {
        clients
            .load_chathistory_targets_timestamp(server, server_time)
            .map(|task| Task::perform(task, Message::Client))
    }

    pub fn overwrite_chathistory_targets_timestamp(
        &self,
        clients: &data::client::Map,
        server: &Server,
        timestamp: DateTime<Utc>,
    ) -> Option<Task<Message>> {
        clients
            .overwrite_chathistory_targets_timestamp(server, timestamp)
            .map(|task| Task::perform(task, Message::Client))
    }

    pub fn get_focused(&self) -> Option<(window::Id, pane_grid::Pane, &Pane)> {
        let Focus { window, pane } = self.focus;
        self.panes
            .get(window, pane)
            .map(|state| (window, pane, state))
    }

    fn get_focused_mut(
        &mut self,
    ) -> Option<(window::Id, pane_grid::Pane, &mut Pane)> {
        let Focus { window, pane } = self.focus;
        self.panes
            .get_mut(window, pane)
            .map(|state| (window, pane, state))
    }

    fn get_focused_with_history_mut(
        &mut self,
    ) -> Option<(
        window::Id,
        pane_grid::Pane,
        &mut Pane,
        &mut history::Manager,
    )> {
        let Focus { window, pane } = self.focus;
        self.panes
            .get_mut(window, pane)
            .map(|state| (window, pane, state, &mut self.history))
    }

    pub fn get_unique_queries(&self, server: &Server) -> Vec<&target::Query> {
        self.history.get_unique_queries(server)
    }

    pub fn refocus_pane(&mut self) -> Task<Message> {
        let Focus { window, pane } = self.focus;

        return self
            .panes
            .iter()
            .find_map(|(w, p, state)| {
                (w == window && p == pane).then(|| {
                    state.buffer.focus().map(move |message| {
                        Message::Pane(
                            window,
                            pane::Message::Buffer(pane, message),
                        )
                    })
                })
            })
            .unwrap_or_else(Task::none);
    }

    fn focus_pane(
        &mut self,
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Task<Message> {
        if (self.focus != Focus { window, pane }) {
            self.focus = Focus { window, pane };

            self.last_changed = Some(Instant::now());

            if window == self.main_window() {
                self.focus_history.push_front(pane);

                self.focus_history.truncate(FOCUS_HISTORY_LEN);

                if self.is_pane_maximized() {
                    self.panes.main.restore();
                    self.panes.main.maximize(pane);
                }
            }
        }

        self.refocus_pane()
    }

    fn focus_first_pane(&mut self, window: window::Id) -> Task<Message> {
        let pane = self
            .panes
            .iter()
            .find_map(|(w, pane, _)| (w == window).then_some(pane));

        pane.map_or(Task::none(), |pane| self.focus_pane(window, pane))
    }

    pub fn focus_window_pane(&mut self, window: window::Id) -> Task<Message> {
        if self.focus.window == window {
            Task::none()
        } else if let Some(pane) = self
            .focus_history
            .front()
            .filter(|_| window == self.main_window())
        {
            self.focus_pane(window, *pane)
        } else {
            self.focus_first_pane(window)
        }
    }

    pub fn focus_window(&mut self, window: window::Id) -> Task<Message> {
        let task = self.focus_window_pane(window);

        window::gain_focus(window).chain(task)
    }

    fn maximize_pane(&mut self) {
        if self.is_pane_maximized() {
            self.panes.main.restore();
        } else if self.focus.window == self.main_window() {
            self.panes.main.maximize(self.focus.pane);
        }
    }

    fn is_pane_maximized(&self) -> bool {
        self.panes.main.maximized().is_some()
    }

    fn new_pane(&mut self, axis: pane_grid::Axis) -> Task<Message> {
        if self.focus.window == self.main_window() {
            // If there is any focused pane on main window, split it
            return self.split_pane(axis);
        } else {
            // If there is no focused pane, split the last pane or create a new empty grid
            let pane =
                self.panes.main.iter().last().map(|(pane, _)| pane).copied();

            if let Some(pane) = pane {
                let result =
                    self.panes.main.split(axis, pane, Pane::new(Buffer::Empty));
                self.last_changed = Some(Instant::now());

                if let Some((pane, _)) = result {
                    return self.focus_pane(self.main_window(), pane);
                }
            } else {
                let (state, pane) =
                    pane_grid::State::new(Pane::new(Buffer::Empty));
                self.panes.main = state;
                self.last_changed = Some(Instant::now());
                return self.focus_pane(self.main_window(), pane);
            }
        }

        Task::none()
    }

    fn split_pane(&mut self, axis: pane_grid::Axis) -> Task<Message> {
        if self.focus.window == self.main_window() {
            let result = self.panes.main.split(
                axis,
                self.focus.pane,
                Pane::new(Buffer::Empty),
            );
            self.last_changed = Some(Instant::now());
            if let Some((pane, _)) = result {
                return self.focus_pane(self.main_window(), pane);
            }
        }

        Task::none()
    }

    fn reset_pane(
        &mut self,
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Task<Message> {
        if let Some(state) = self.panes.get_mut(window, pane) {
            state.buffer.reset();
        }

        Task::none()
    }

    fn close_pane(
        &mut self,
        clients: &mut data::client::Map,
        config: &Config,
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Task<Message> {
        if let Some(state) = self.panes.get(window, pane) {
            mark_as_read_on_buffer_close(
                &state.buffer,
                &mut self.history,
                clients,
                config,
            );
        }

        self.last_changed = Some(Instant::now());

        if window == self.main_window() {
            self.focus_history = self
                .focus_history
                .iter()
                .filter(|p| **p != pane)
                .copied()
                .collect();

            if let Some((_, sibling)) = self.panes.main.close(pane) {
                if (Focus { window, pane } == self.focus) {
                    return self.focus_pane(self.main_window(), sibling);
                }
            } else if let Some(pane) = self.panes.main.get_mut(pane) {
                pane.buffer = Buffer::Empty;
            }
        } else if self.panes.popout.remove(&window).is_some() {
            return window::close(window)
                .chain(self.focus_window(self.main_window()));
        }

        Task::none()
    }

    fn popout_pane(
        &mut self,
        clients: &mut data::client::Map,
        config: &Config,
    ) -> Task<Message> {
        let Focus { pane, .. } = self.focus;

        self.focus_history = self
            .focus_history
            .clone()
            .into_iter()
            .filter(|p| *p != pane)
            .collect();

        if let Some((pane, _)) = self.panes.main.close(pane)
            && let Some(buffer) = pane.buffer.data()
        {
            return self.open_buffer(
                buffer,
                BufferAction::NewWindow,
                clients,
                config,
            );
        }

        Task::none()
    }

    fn merge_pane(
        &mut self,
        clients: &mut data::client::Map,
        config: &Config,
    ) -> Task<Message> {
        let Focus { window, pane } = self.focus;

        if let Some(pane) = self
            .panes
            .popout
            .remove(&window)
            .and_then(|panes| panes.get(pane).cloned())
        {
            let task = match pane.buffer.data() {
                Some(buffer) => self.open_buffer(
                    buffer,
                    BufferAction::NewPane,
                    clients,
                    config,
                ),
                None => self.new_pane(pane_grid::Axis::Horizontal),
            };

            return Task::batch(vec![
                window::close(window),
                window::gain_focus(self.main_window()).chain(task),
            ]);
        }

        Task::none()
    }

    fn swap_pane_with_focus(
        &mut self,
        from_window: window::Id,
        from_pane: pane_grid::Pane,
    ) -> Task<Message> {
        self.last_changed = Some(Instant::now());

        let Focus {
            window: to_window,
            pane: to_pane,
        } = self.focus;

        if from_window == self.main_window() && to_window == self.main_window()
        {
            self.panes.main.swap(from_pane, to_pane);

            self.focus_pane(from_window, from_pane)
        } else {
            if let Some((from_state, to_state)) = self
                .panes
                .get(from_window, from_pane)
                .cloned()
                .zip(self.panes.get(to_window, to_pane).cloned())
            {
                if let Some(state) = self.panes.get_mut(from_window, from_pane)
                {
                    *state = to_state;
                }
                if let Some(state) = self.panes.get_mut(to_window, to_pane) {
                    *state = from_state;
                }
            }

            Task::none()
        }
    }

    pub fn track(
        &mut self,
        clients: Option<&data::client::Map>,
    ) -> Task<Message> {
        let resources = self.panes.resources().collect();

        Task::batch(
            self.history
                .track(resources, clients)
                .into_iter()
                .map(|fut| Task::perform(fut, Message::History))
                .collect::<Vec<_>>(),
        )
    }

    pub fn tick(
        &mut self,
        now: Instant,
        clients: &data::client::Map,
    ) -> Task<Message> {
        let history = Task::batch(
            self.history
                .tick(now.into(), clients)
                .into_iter()
                .map(|task| Task::perform(task, Message::History))
                .collect::<Vec<_>>(),
        );

        if let Some(last_changed) = self.last_changed
            && now.duration_since(last_changed) >= SAVE_AFTER
        {
            let dashboard = data::Dashboard::from(&*self);

            self.last_changed = None;

            return Task::batch(vec![
                Task::perform(dashboard.save(), Message::DashboardSaved),
                history,
            ]);
        }

        history
    }

    pub fn toggle_command_bar(
        &mut self,
        buffers: &[buffer::Upstream],
        version: &Version,
        config: &Config,
        theme: &mut Theme,
    ) -> Task<Message> {
        if self.command_bar.is_some() {
            // Remove theme preview
            *theme = theme.selected();

            self.close_command_bar();
            // Refocus the pane so text input gets refocused
            let Focus { window, pane } = self.focus;
            self.focus_pane(window, pane)
        } else {
            self.open_command_bar(buffers, version, config);
            Task::none()
        }
    }

    fn open_command_bar(
        &mut self,
        buffers: &[buffer::Upstream],
        version: &Version,
        config: &Config,
    ) {
        self.command_bar = Some(CommandBar::new(
            buffers,
            version,
            config,
            self.focus,
            self.buffer_resize_action(),
            self.main_window(),
        ));
    }

    fn close_command_bar(&mut self) {
        self.command_bar = None;
    }

    fn buffer_resize_action(&self) -> data::buffer::Resize {
        let can_resize_buffer =
            self.focus.window == self.main_window() && self.panes.len() > 1;
        data::buffer::Resize::action(
            can_resize_buffer,
            self.is_pane_maximized(),
        )
    }

    pub fn receive_file_transfer(
        &mut self,
        server: &Server,
        casemapping: isupport::CaseMap,
        request: file_transfer::ReceiveRequest,
        config: &Config,
    ) -> Option<Task<Message>> {
        if !config.file_transfer.enabled {
            log::info!(
                "file transfer request from {} ignored",
                request.from.formatted(UsernameFormat::Full)
            );

            return None;
        }

        let event = self.file_transfers.receive(request.clone(), config)?;

        self.notifications.notify(
            &config.notifications,
            &Notification::FileTransferRequest {
                nick: request.from.nickname().to_owned(),
                filename: match event {
                    file_transfer::manager::Event::NewTransfer(
                        ref transfer,
                        _,
                    ) => transfer.filename.clone(),
                },
            },
            server,
        );

        let query = target::Query::from(request.from);

        Some(self.handle_file_transfer_event(
            server,
            casemapping,
            &query,
            event,
            &config.buffer,
        ))
    }

    pub fn handle_file_transfer_event(
        &mut self,
        server: &Server,
        casemapping: isupport::CaseMap,
        query: &target::Query,
        event: file_transfer::manager::Event,
        buffer_config: &config::Buffer,
    ) -> Task<Message> {
        let mut tasks = vec![];

        match event {
            file_transfer::manager::Event::NewTransfer(transfer, task) => {
                match transfer.direction {
                    file_transfer::Direction::Received => {
                        tasks.push(self.record_message(
                            server,
                            casemapping,
                            data::Message::file_transfer_request_received(
                                &transfer.remote_user,
                                query,
                                &transfer.filename,
                            ),
                            buffer_config,
                        ));
                    }
                    file_transfer::Direction::Sent => {
                        tasks.push(self.record_message(
                            server,
                            casemapping,
                            data::Message::file_transfer_request_sent(
                                &transfer.remote_user,
                                query,
                                &transfer.filename,
                            ),
                            buffer_config,
                        ));
                    }
                }

                tasks.push(Task::run(task, Message::FileTransfer));
            }
        }

        Task::batch(tasks)
    }

    fn from_data(
        data: data::Dashboard,
        config: &Config,
        main_window: &Window,
    ) -> (Self, Task<Message>) {
        use pane_grid::Configuration;

        fn configuration(
            pane: data::Pane,
            config: &Config,
        ) -> Configuration<Pane> {
            match pane {
                data::Pane::Split { axis, ratio, a, b } => {
                    Configuration::Split {
                        axis: match axis {
                            data::pane::Axis::Horizontal => {
                                pane_grid::Axis::Horizontal
                            }
                            data::pane::Axis::Vertical => {
                                pane_grid::Axis::Vertical
                            }
                        },
                        ratio,
                        a: Box::new(configuration(*a, config)),
                        b: Box::new(configuration(*b, config)),
                    }
                }
                data::Pane::Buffer { buffer } => {
                    Configuration::Pane(Pane::new(Buffer::from_data(
                        buffer,
                        Size::default(),
                        config,
                    )))
                }
                data::Pane::Empty => {
                    Configuration::Pane(Pane::new(Buffer::empty()))
                }
            }
        }

        let panes = Panes {
            main_window: main_window.id,
            main: pane_grid::State::with_configuration(configuration(
                data.pane, config,
            )),
            popout: HashMap::new(),
        };

        let focus = panes
            .iter()
            // This should never fail
            .find_map(|(window, pane, state)| {
                (state.buffer.data() == data.focus_buffer)
                    .then_some(Focus { window, pane })
            })
            // But if somehow it does, we just focus the "first" pane from the main window
            .unwrap_or_else(|| {
                let (_, pane) = pane_grid::State::new(());

                Focus {
                    window: main_window.id,
                    pane,
                }
            });

        let mut dashboard = Self {
            panes,
            focus,
            focus_history: VecDeque::from([focus.pane]),
            side_menu: Sidebar::new(),
            history: history::Manager::default(),
            last_changed: None,
            command_bar: None,
            file_transfers: file_transfer::Manager::default(),
            theme_editor: None,
            notifications: notification::Notifications::new(config),
            previews: preview::Collection::default(),
            buffer_settings: data.buffer_settings.clone(),
        };

        let mut tasks = vec![];

        for pane in data.popout_panes {
            // Popouts are only a single pane
            let Configuration::Pane(pane) = configuration(pane, config) else {
                continue;
            };

            if let Some(buffer) = pane.buffer.data() {
                tasks.push(dashboard.open_buffer(
                    buffer,
                    BufferAction::NewWindow,
                    &mut data::client::Map::default(),
                    config,
                ));
            }
        }

        let tasks = Task::batch(tasks)
            .chain(dashboard.focus_pane(focus.window, focus.pane));

        (dashboard, tasks)
    }

    pub fn history(&self) -> &history::Manager {
        &self.history
    }

    pub fn get_filters(&mut self) -> &mut Vec<Filter> {
        self.history.get_filters()
    }

    pub fn handle_window_event(
        &mut self,
        id: window::Id,
        event: window::Event,
        theme: &mut Theme,
    ) -> Task<Message> {
        if self.panes.popout.contains_key(&id) {
            match event {
                window::Event::CloseRequested => {
                    self.panes.popout.remove(&id);
                    return window::close(id);
                }
                window::Event::Focused => {
                    return self.focus_window_pane(id);
                }
                window::Event::Moved(_)
                | window::Event::Resized(_)
                | window::Event::Unfocused
                | window::Event::Opened { .. } => {}
            }
        } else if self.theme_editor.as_ref().is_some_and(|e| e.window == id) {
            match event {
                window::Event::CloseRequested => {
                    if let Some(editor) = self.theme_editor.take() {
                        *theme = theme.selected();
                        return window::close(editor.window);
                    }
                }
                window::Event::Moved(_)
                | window::Event::Resized(_)
                | window::Event::Focused
                | window::Event::Unfocused
                | window::Event::Opened { .. } => {}
            }
        }

        Task::none()
    }

    pub fn preview_theme_in_editor(
        &mut self,
        styles: theme::Styles,
        main_window: &Window,
        theme: &mut Theme,
        config: &Config,
    ) -> Task<Message> {
        *theme = theme.preview(data::Theme::new("Custom Theme".into(), styles));

        if let Some(editor) = &self.theme_editor {
            window::gain_focus(editor.window)
        } else {
            let (editor, task) = ThemeEditor::open(main_window, config);

            self.theme_editor = Some(editor);

            task.then(|_| Task::none())
        }
    }

    pub fn exit(
        &mut self,
        clients: &mut data::client::Map,
        config: &Config,
    ) -> Task<Message> {
        if config.buffer.mark_as_read.on_application_exit {
            self.history.kinds()
        } else {
            self.panes
                .iter()
                .filter_map(|(_, _, state)| {
                    if config
                        .buffer
                        .mark_as_read
                        .on_buffer_close
                        .mark_as_read(state.buffer.is_scrolled_to_bottom())
                    {
                        state.buffer.data().and_then(history::Kind::from_buffer)
                    } else {
                        None
                    }
                })
                .collect()
        }
        .into_iter()
        .for_each(|kind| {
            mark_as_read(kind, &mut self.history, clients, TokenPriority::High);
        });

        let history = self.history.exit(clients);
        let last_changed = self.last_changed.take();
        let dashboard = data::Dashboard::from(&*self);

        Task::perform(
            async move {
                if last_changed.is_some() {
                    match dashboard.save().await {
                        Ok(()) => {
                            log::debug!("dashboard saved");
                        }
                        Err(error) => {
                            log::warn!("error saving dashboard: {error}");
                        }
                    }
                }

                history.await
            },
            Message::History,
        )
    }

    fn open_channel(
        &mut self,
        server: Server,
        channel: target::Channel,
        clients: &mut data::client::Map,
        buffer_action: BufferAction,
        config: &Config,
    ) -> Task<Message> {
        let buffer = buffer::Upstream::Channel(server.clone(), channel.clone());

        // Need to join channel
        if !clients.contains_channel(&server, &channel) {
            clients.join(&server, slice::from_ref(&channel));
        }

        self.open_buffer(
            data::Buffer::Upstream(buffer),
            buffer_action,
            clients,
            config,
        )
    }

    pub fn open_target(
        &mut self,
        server: Server,
        target: Target,
        clients: &mut data::client::Map,
        buffer_action: BufferAction,
        config: &Config,
    ) -> Task<Message> {
        match target {
            Target::Channel(channel) => self.open_channel(
                server,
                channel,
                clients,
                buffer_action,
                config,
            ),
            Target::Query(query) => {
                let buffer = data::Buffer::Upstream(buffer::Upstream::Query(
                    server, query,
                ));

                self.open_buffer(buffer.clone(), buffer_action, clients, config)
            }
        }
    }

    fn main_window(&self) -> window::Id {
        self.panes.main_window
    }
}

fn mark_server_as_read(
    server: Server,
    history: &mut history::Manager,
    clients: &mut data::client::Map,
) {
    for kind in history.server_kinds(server) {
        mark_as_read(kind, history, clients, TokenPriority::User);
    }
}

fn mark_as_read(
    kind: history::Kind,
    history: &mut history::Manager,
    clients: &mut data::client::Map,
    priority: TokenPriority,
) {
    let read_marker = history.mark_as_read(&kind);

    if let (Some(server), Some(target), Some(read_marker)) =
        (kind.server(), kind.target(), read_marker)
    {
        clients.send_markread(server, target, read_marker, priority);
    }
}

fn mark_as_read_on_buffer_close(
    buffer: &Buffer,
    history: &mut history::Manager,
    clients: &mut data::client::Map,
    config: &Config,
) {
    if config
        .buffer
        .mark_as_read
        .on_buffer_close
        .mark_as_read(buffer.is_scrolled_to_bottom())
        && let Some(kind) = buffer.data().and_then(history::Kind::from_buffer)
    {
        mark_as_read(kind, history, clients, TokenPriority::High);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Focus {
    pub window: window::Id,
    pub pane: pane_grid::Pane,
}

impl<'a> From<&'a Dashboard> for data::Dashboard {
    fn from(dashboard: &'a Dashboard) -> Self {
        use pane_grid::Node;

        fn from_layout(
            panes: &pane_grid::State<Pane>,
            node: pane_grid::Node,
        ) -> data::Pane {
            match node {
                Node::Split {
                    axis, ratio, a, b, ..
                } => data::Pane::Split {
                    axis: match axis {
                        pane_grid::Axis::Horizontal => {
                            data::pane::Axis::Horizontal
                        }
                        pane_grid::Axis::Vertical => data::pane::Axis::Vertical,
                    },
                    ratio,
                    a: Box::new(from_layout(panes, *a)),
                    b: Box::new(from_layout(panes, *b)),
                },
                Node::Pane(pane) => panes
                    .get(pane)
                    .cloned()
                    .map_or(data::Pane::Empty, data::Pane::from),
            }
        }

        let layout = dashboard.panes.main.layout().clone();
        let focus = dashboard.focus;

        data::Dashboard {
            pane: from_layout(&dashboard.panes.main, layout),
            popout_panes: dashboard
                .panes
                .popout
                .values()
                .map(|state| from_layout(state, state.layout().clone()))
                .collect(),
            buffer_settings: dashboard.buffer_settings.clone(),
            focus_buffer: dashboard.panes.iter().find_map(|(w, p, state)| {
                (w == focus.window && p == focus.pane)
                    .then_some(state.buffer.data())
                    .flatten()
            }),
        }
    }
}

#[derive(Clone)]
pub struct Panes {
    main_window: window::Id,
    main: pane_grid::State<Pane>,
    popout: HashMap<window::Id, pane_grid::State<Pane>>,
}

impl Panes {
    fn len(&self) -> usize {
        self.main.panes.len() + self.popout.len()
    }

    fn get(&self, window: window::Id, pane: pane_grid::Pane) -> Option<&Pane> {
        if self.main_window == window {
            self.main.get(pane)
        } else {
            self.popout.get(&window).and_then(|panes| panes.get(pane))
        }
    }

    fn get_mut(
        &mut self,
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Option<&mut Pane> {
        if self.main_window == window {
            self.main.get_mut(pane)
        } else {
            self.popout
                .get_mut(&window)
                .and_then(|panes| panes.get_mut(pane))
        }
    }

    fn get_mut_by_buffer(
        &mut self,
        buffer: &data::Buffer,
    ) -> Option<(window::Id, pane_grid::Pane, &mut Pane)> {
        self.iter_mut().find(|(_, _, state)| {
            state.buffer.data().is_some_and(|b| b == *buffer)
        })
    }

    fn iter(
        &self,
    ) -> impl Iterator<Item = (window::Id, pane_grid::Pane, &Pane)> {
        self.main
            .iter()
            .map(move |(pane, state)| (self.main_window, *pane, state))
            .chain(self.popout.iter().flat_map(|(window_id, panes)| {
                panes.iter().map(|(pane, state)| (*window_id, *pane, state))
            }))
    }

    fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (window::Id, pane_grid::Pane, &mut Pane)> {
        let main_window = self.main_window;

        self.main
            .iter_mut()
            .map(move |(pane, state)| (main_window, *pane, state))
            .chain(self.popout.iter_mut().flat_map(|(window_id, panes)| {
                panes
                    .iter_mut()
                    .map(|(pane, state)| (*window_id, *pane, state))
            }))
    }

    fn resources(&self) -> impl Iterator<Item = data::history::Resource> + '_ {
        self.main.panes.values().filter_map(Pane::resource).chain(
            self.popout.values().flat_map(|state| {
                state.panes.values().filter_map(Pane::resource)
            }),
        )
    }

    fn visible_urls(&self) -> HashSet<url::Url> {
        self.main
            .panes
            .values()
            .flat_map(Pane::visible_urls)
            .chain(self.popout.values().flat_map(|state| {
                state.panes.values().flat_map(Pane::visible_urls)
            }))
            .cloned()
            .collect()
    }
}

fn all_buffers(
    clients: &client::Map,
    history: &history::Manager,
) -> Vec<buffer::Upstream> {
    clients
        .connected_servers()
        .flat_map(|server| {
            std::iter::once(buffer::Upstream::Server(server.clone()))
                .chain(clients.get_channels(server).map(|channel| {
                    buffer::Upstream::Channel(server.clone(), channel.clone())
                }))
                .chain(history.get_unique_queries(server).into_iter().map(
                    |nick| {
                        buffer::Upstream::Query(server.clone(), nick.clone())
                    },
                ))
        })
        .collect()
}

fn all_buffers_with_has_unread(
    clients: &client::Map,
    history: &history::Manager,
) -> Vec<(buffer::Upstream, bool)> {
    clients
        .connected_servers()
        .flat_map(|server| {
            std::iter::once((
                buffer::Upstream::Server(server.clone()),
                history.has_unread(&history::Kind::Server(server.clone())),
            ))
            .chain(clients.get_channels(server).map(|channel| {
                (
                    buffer::Upstream::Channel(server.clone(), channel.clone()),
                    history.has_unread(&history::Kind::Channel(
                        server.clone(),
                        channel.clone(),
                    )),
                )
            }))
            .chain(
                history.get_unique_queries(server).into_iter().map(|nick| {
                    (
                        buffer::Upstream::Query(server.clone(), nick.clone()),
                        history.has_unread(&history::Kind::Query(
                            server.clone(),
                            nick.clone(),
                        )),
                    )
                }),
            )
        })
        .collect()
}

fn open_buffers(dashboard: &Dashboard) -> Vec<buffer::Upstream> {
    dashboard
        .panes
        .iter()
        .filter_map(|(_, _, pane)| pane.buffer.upstream())
        .cloned()
        .collect()
}

fn closed_buffers(
    dashboard: &Dashboard,
    clients: &client::Map,
) -> Vec<buffer::Upstream> {
    let open_buffers = open_buffers(dashboard);

    all_buffers(clients, &dashboard.history)
        .into_iter()
        .filter(|buffer| !open_buffers.contains(buffer))
        .collect()
}

fn cycle_next_buffer(
    current: Option<&buffer::Upstream>,
    mut all: Vec<buffer::Upstream>,
    opened: &[buffer::Upstream],
) -> Option<buffer::Upstream> {
    all.retain(|buffer| Some(buffer) == current || !opened.contains(buffer));

    let next = || {
        let buffer = current?;
        let index = all.iter().position(|b| b == buffer)?;
        all.get(index + 1)
    };

    next().or_else(|| all.first()).cloned()
}

fn cycle_previous_buffer(
    current: Option<&buffer::Upstream>,
    mut all: Vec<buffer::Upstream>,
    opened: &[buffer::Upstream],
) -> Option<buffer::Upstream> {
    all.retain(|buffer| Some(buffer) == current || !opened.contains(buffer));

    let previous = || {
        let buffer = current?;
        let index = all.iter().position(|b| b == buffer).filter(|i| *i > 0)?;

        all.get(index - 1)
    };

    previous().or_else(|| all.last()).cloned()
}

fn cycle_next_unread_buffer(
    current: Option<&buffer::Upstream>,
    mut all: Vec<(buffer::Upstream, bool)>,
    opened: &[buffer::Upstream],
) -> Option<buffer::Upstream> {
    all.retain(|(buffer, _)| {
        Some(buffer) == current || !opened.contains(buffer)
    });

    let buffer = current?;

    let index = all.iter().position(|(b, _)| b == buffer)?;

    let next_after = || {
        all.iter()
            .skip(index + 1)
            .find_map(|(b, has_unread)| has_unread.then_some(b))
    };

    let next_before = || {
        all.iter()
            .take(index)
            .find_map(|(b, has_unread)| has_unread.then_some(b))
    };

    next_after().or_else(|| next_before().or(None)).cloned()
}

fn cycle_previous_unread_buffer(
    current: Option<&buffer::Upstream>,
    mut all: Vec<(buffer::Upstream, bool)>,
    opened: &[buffer::Upstream],
) -> Option<buffer::Upstream> {
    all.retain(|(buffer, _)| {
        Some(buffer) == current || !opened.contains(buffer)
    });

    let buffer = current?;

    let index = all.iter().rev().position(|(b, _)| b == buffer)?;

    let previous_before = || {
        all.iter()
            .rev()
            .skip(index + 1)
            .find_map(|(b, has_unread)| has_unread.then_some(b))
    };

    let previous_after = || {
        all.iter()
            .rev()
            .take(index)
            .find_map(|(b, has_unread)| has_unread.then_some(b))
    };

    previous_before()
        .or_else(|| previous_after().or(None))
        .cloned()
}
