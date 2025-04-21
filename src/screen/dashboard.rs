use std::collections::{HashMap, HashSet, VecDeque, hash_map};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{convert, slice};

use chrono::{DateTime, Utc};
use data::dashboard::{self, BufferAction};
use data::environment::{RELEASE_WEBSITE, WIKI_WEBSITE};
use data::history::ReadMarker;
use data::history::manager::Broadcast;
use data::isupport::{self, ChatHistorySubcommand, MessageReference};
use data::target::{self, Target};
use data::user::Nick;
use data::{
    Config, Notification, Server, Version, client, command, config,
    environment, file_transfer, history, preview,
};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{Space, column, container, row};
use iced::window::get_position;
use iced::{Length, Task, Vector, clipboard};
use log::{debug, error};

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
    SelectedText(Vec<(f32, String)>),
    History(history::manager::Message),
    DashboardSaved(Result<(), data::dashboard::Error>),
    Task(command_bar::Message),
    Shortcut(shortcut::Command),
    FileTransfer(file_transfer::task::Update),
    SendFileSelected(Server, Nick, Option<PathBuf>),
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
    QuitServer(Server),
    IrcError(anyhow::Error),
    Exit,
    OpenUrl(String, bool),
    ImagePreview(PathBuf, url::Url),
}

impl Dashboard {
    pub fn empty(
        config: &Config,
        main_window: &Window,
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
            file_transfers: file_transfer::Manager::new(
                config.file_transfer.clone(),
            ),
            theme_editor: None,
            notifications: notification::Notifications::new(),
            previews: preview::Collection::default(),
            buffer_settings: dashboard::BufferSettings::default(),
        };

        let command = dashboard.track(config);

        (dashboard, command)
    }

    pub fn restore(
        dashboard: data::Dashboard,
        config: &Config,
        main_window: &Window,
    ) -> (Self, Task<Message>) {
        let (mut dashboard, task) =
            Dashboard::from_data(dashboard, config, main_window);

        let tasks = Task::batch(vec![task, dashboard.track(config)]);

        (dashboard, tasks)
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
                            self.close_pane(self.focus.window, self.focus.pane),
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

                            match event {
                                buffer::Event::UserContext(event) => {
                                    match event {
                                        buffer::user_context::Event::ToggleAccessLevel(
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
                                                                            clients.send(&input.buffer, encoded);
                                                                        }
                                                                    }
                                                                    buffer::user_context::Event::SendWhois(server, nick) => {
                                                                        let buffer =
                                                                            pane.buffer.upstream().cloned().unwrap_or_else(
                                                                                || buffer::Upstream::Server(server.clone()),
                                                                            );

                                                                        let command =
                                                                            command::Irc::Whois(None, nick.to_string());

                                                                        let input =
                                                                            data::Input::command(buffer.clone(), command);

                                                                        if let Some(encoded) = input.encoded() {
                                                                            clients.send(&input.buffer, encoded);
                                                                        }

                                                                        if let Some(nick) = clients.nickname(buffer.server()) {
                                                                            let mut user = nick.to_owned().into();
                                                                            let mut channel_users = &[][..];
                                                                            let chantypes =
                                                                                clients.get_chantypes(buffer.server());
                                                                            let statusmsg =
                                                                                clients.get_statusmsg(buffer.server());
                                                                            let casemapping =
                                                                                clients.get_casemapping(buffer.server());

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
                                                                                config,
                                                                            ) {
                                                                                let mut tasks = vec![task];

                                                                                for message in messages {
                                                                                    if let Some(task) = self
                                                                                        .history
                                                                                        .record_message(input.server(), message)
                                                                                    {
                                                                                        tasks.push(Task::perform(
                                                                                            task,
                                                                                            Message::History,
                                                                                        ));
                                                                                    }
                                                                                }

                                                                                return (Task::batch(tasks), None);
                                                                            }
                                                                        }
                                                                    }
                                                                    buffer::user_context::Event::OpenQuery(
                                                                        server,
                                                                        query,
                                                                        buffer_action,
                                                                    ) => {
                                                                        let buffer = buffer::Upstream::Query(server, query);
                                                                        return (
                                                                            Task::batch(vec![
                                                                                task,
                                                                                self.open_buffer(
                                                                                    data::Buffer::Upstream(buffer),
                                                                                    buffer_action,
                                                                                    config,
                                                                                ),
                                                                            ]),
                                                                            None,
                                                                        );
                                                                    }
                                                                    buffer::user_context::Event::InsertNickname(nick) => {
                                                                        let Some((_, pane, history)) =
                                                                            self.get_focused_with_history_mut()
                                                                        else {
                                                                            return (task, None);
                                                                        };

                                            return (
                                                Task::batch(vec![
                                                    task,
                                                    pane.buffer
                                                        .insert_user_to_input(nick, history, &config.buffer.text_input.autocomplete)
                                                        .map(move |message| {
                                                            Message::Pane(
                                                                window,
                                                                pane::Message::Buffer(id, message),
                                                            )
                                                        }),
                                                ]),
                                                None,
                                            );
                                        }
                                        buffer::user_context::Event::SendFile(server, nick) => {
                                            return (
                                                Task::batch(vec![
                                                    task,
                                                    Task::perform(
                                                        async move {
                                                            rfd::AsyncFileDialog::new()
                                                                .pick_file()
                                                                .await
                                                                .map(|handle| {
                                                                    handle.path().to_path_buf()
                                                                })
                                                        },
                                                        move |file| {
                                                            Message::SendFileSelected(
                                                                server.clone(),
                                                                nick.clone(),
                                                                file,
                                                            )
                                                        },
                                                    ),
                                                ]),
                                                None,
                                            );
                                        }
                                        buffer::user_context::Event::CtcpRequest(
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
                                                clients.send(&input.buffer, encoded);
                                            }

                                            return (Task::none(), None);
                                        }
                                    }
                                }
                                buffer::Event::OpenBuffers(targets) => {
                                    let mut tasks = vec![task];

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
                                buffer::Event::History(history_task) => {
                                    return (
                                        Task::batch(vec![
                                            task,
                                            history_task.map(Message::History),
                                        ]),
                                        None,
                                    );
                                }
                                buffer::Event::GoToMessage(
                                    server,
                                    channel,
                                    message,
                                ) => {
                                    let buffer = data::Buffer::Upstream(
                                        buffer::Upstream::Channel(
                                            server, channel,
                                        ),
                                    );

                                    let mut tasks = vec![];

                                    if self
                                        .panes
                                        .get_mut_by_buffer(&buffer)
                                        .is_none()
                                    {
                                        tasks.push(
                                            self.open_buffer(
                                                buffer.clone(),
                                                config
                                                    .actions
                                                    .buffer
                                                    .click_highlight,
                                                config,
                                            ),
                                        );
                                    }

                                    if let Some((window, pane, state)) =
                                        self.panes.get_mut_by_buffer(&buffer)
                                    {
                                        tasks.push(
                                            state
                                                .buffer
                                                .scroll_to_message(
                                                    message,
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
                                        );
                                    }

                                    return (Task::batch(tasks), None);
                                }
                                buffer::Event::RequestOlderChatHistory => {
                                    if let Some(buffer) = pane.buffer.data() {
                                        self.request_older_chathistory(
                                            clients, &buffer,
                                        );
                                    }
                                }
                                buffer::Event::PreviewChanged => {
                                    let visible = self.panes.visible_urls();
                                    let tracking = self
                                        .previews
                                        .keys()
                                        .cloned()
                                        .collect::<HashSet<_>>();
                                    let missing = visible
                                        .difference(&tracking)
                                        .cloned()
                                        .collect::<Vec<_>>();
                                    let removed = tracking.difference(&visible);

                                    for url in &missing {
                                        self.previews.insert(
                                            url.clone(),
                                            preview::State::Loading,
                                        );
                                    }

                                    for url in removed {
                                        self.previews.remove(url);
                                    }

                                    return (
                                        Task::batch(missing.into_iter().map(
                                            |url| {
                                                Task::perform(
                                                    data::preview::load(
                                                        url.clone(),
                                                        config.preview.clone(),
                                                    ),
                                                    move |result| {
                                                        Message::LoadPreview((
                                                            url.clone(),
                                                            result,
                                                        ))
                                                    },
                                                )
                                            },
                                        )),
                                        None,
                                    );
                                }
                                buffer::Event::HidePreview(kind, hash, url) => {
                                    self.history.hide_preview(kind, hash, url);
                                }
                                buffer::Event::MarkAsRead(kind) => {
                                    self.mark_as_read(kind, clients);
                                }
                                buffer::Event::OpenUrl(url) => {
                                    return (
                                        Task::none(),
                                        Some(Event::OpenUrl(
                                            url,
                                            config
                                                .buffer
                                                .url
                                                .prompt_before_open,
                                        )),
                                    );
                                }
                                buffer::Event::ImagePreview(path, url) => {
                                    return (
                                        Task::none(),
                                        Some(Event::ImagePreview(path, url)),
                                    );
                                }
                            }

                            return (task, None);
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
                                settings.channel.topic.toggle_visibility();
                            }

                            self.last_changed = Some(Instant::now());
                            return (Task::none(), None);
                        }
                    }
                    pane::Message::MaximizePane => self.maximize_pane(),
                    pane::Message::Popout => {
                        return (self.popout_pane(config), None);
                    }
                    pane::Message::Merge => {
                        return (self.merge_pane(config), None);
                    }
                    pane::Message::ScrollToBottom => {
                        let Focus { window, pane } = self.focus;

                        if let Some(state) = self.panes.get_mut(window, pane) {
                            let mut task = state.buffer.scroll_to_end().map(
                                move |message| {
                                    Message::Pane(
                                        window,
                                        pane::Message::Buffer(pane, message),
                                    )
                                },
                            );

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
                        if let Some((_, _, pane)) = self.get_focused_mut() {
                            if let Some(kind) = pane
                                .buffer
                                .data()
                                .and_then(history::Kind::from_buffer)
                            {
                                self.mark_as_read(kind, clients);
                            }
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
                    sidebar::Event::New(buffer) => (
                        self.open_buffer(
                            data::Buffer::Upstream(buffer),
                            BufferAction::NewPane,
                            config,
                        ),
                        None,
                    ),
                    sidebar::Event::Popout(buffer) => (
                        self.open_buffer(
                            data::Buffer::Upstream(buffer),
                            BufferAction::NewWindow,
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
                            config,
                        ),
                        None,
                    ),
                    sidebar::Event::Close(window, pane) => {
                        (self.close_pane(window, pane), None)
                    }
                    sidebar::Event::Swap(window, pane) => {
                        (self.swap_pane_with_focus(window, pane), None)
                    }
                    sidebar::Event::Leave(buffer) => self.leave_buffer(
                        clients,
                        buffer,
                        config.buffer.mark_as_read.on_buffer_close,
                    ),
                    sidebar::Event::ToggleInternalBuffer(buffer) => {
                        (self.toggle_internal_buffer(config, buffer), None)
                    }
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
                    sidebar::Event::ToggleThemeEditor => {
                        (self.toggle_theme_editor(theme, main_window), None)
                    }
                    sidebar::Event::OpenDocumentation => {
                        let _ = open::that_detached(WIKI_WEBSITE);
                        (Task::none(), None)
                    }
                    sidebar::Event::MarkAsRead(buffer) => {
                        if let Some(kind) = history::Kind::from_buffer(
                            data::Buffer::Upstream(buffer),
                        ) {
                            self.mark_as_read(kind, clients);
                        }

                        (Task::none(), None)
                    }
                    sidebar::Event::OpenConfigFile => {
                        let _ = open::that_detached(Config::path());
                        (Task::none(), None)
                    }
                };

                return (
                    Task::batch(vec![
                        event_task,
                        command.map(Message::Sidebar),
                    ]),
                    event,
                );
            }
            Message::SelectedText(contents) => {
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
                    return (clipboard::write(contents), None);
                }
            }
            Message::History(message) => {
                if let Some(event) = self.history.update(message) {
                    match event {
                        history::manager::Event::Loaded(kind) => {
                            let buffer = kind.into();

                            if let Some((window, pane, state)) =
                                self.panes.get_mut_by_buffer(&buffer)
                            {
                                return (
                                    state
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
                                    None,
                                );
                            }
                        }
                        history::manager::Event::Closed(kind, read_marker) => {
                            if let (
                                Some(server),
                                Some(target),
                                Some(read_marker),
                            ) = (kind.server(), kind.target(), read_marker)
                            {
                                if let Err(e) = clients.send_markread(
                                    server,
                                    target,
                                    read_marker,
                                ) {
                                    return (
                                        Task::none(),
                                        Some(Event::IrcError(e)),
                                    );
                                };
                            }
                        }
                        history::manager::Event::Exited(results) => {
                            for (kind, read_marker) in results {
                                if let (
                                    Some(server),
                                    Some(target),
                                    Some(read_marker),
                                ) =
                                    (kind.server(), kind.target(), read_marker)
                                {
                                    if let Err(e) = clients.send_markread(
                                        server,
                                        target,
                                        read_marker,
                                    ) {
                                        return (
                                            Task::none(),
                                            Some(Event::IrcError(e)),
                                        );
                                    };
                                }
                            }

                            return (Task::none(), Some(Event::Exit));
                        }
                    }
                }
            }
            Message::DashboardSaved(Ok(())) => {
                log::info!("dashboard saved");
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
                                    (self.close_pane(window, pane), None)
                                }
                                command_bar::Buffer::Replace(buffer) => (
                                    self.open_buffer(
                                        data::Buffer::Upstream(buffer),
                                        BufferAction::ReplacePane,
                                        config,
                                    ),
                                    None,
                                ),
                                command_bar::Buffer::Popout => (self.popout_pane(config), None),
                                command_bar::Buffer::Merge => (self.merge_pane(config), None),
                                command_bar::Buffer::ToggleInternal(buffer) => {
                                    (self.toggle_internal_buffer(config, buffer), None)
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
                            command_bar::Command::UI(command) => match command {
                                command_bar::Ui::ToggleSidebarVisibility => {
                                    self.side_menu.toggle_visibility();
                                    (Task::none(), None)
                                }
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
                                        let (editor, task) = ThemeEditor::open(main_window);

                                        self.theme_editor = Some(editor);

                                        (task.then(|_| Task::none()), None)
                                    }
                                }
                                command_bar::Theme::OpenThemesWebsite => {
                                    let _ = open::that_detached(environment::THEME_WEBSITE);
                                    (Task::none(), None)
                                }
                            },
                            command_bar::Command::Window(command) => match command {
                                command_bar::Window::ToggleFullscreen => {
                                    (window::toggle_fullscreen(), None)
                                }
                            },
                            command_bar::Command::Application(application) => match application {
                                command_bar::Application::Quit => (self.exit(config), None),
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

                    if window == self.main_window() {
                        if let Some(adjacent) =
                            self.panes.main.adjacent(pane, direction)
                        {
                            return self.focus_pane(window, adjacent);
                        }
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
                        return (self.close_pane(window, pane), None);
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

                        if let Some((window, pane, state)) =
                            self.get_focused_mut()
                        {
                            if let Some(buffer) = cycle_next_buffer(
                                state.buffer.upstream(),
                                all_buffers,
                                &open_buffers,
                            ) {
                                state.buffer = Buffer::from(
                                    data::Buffer::Upstream(buffer),
                                );
                                self.last_changed = Some(Instant::now());
                                return (self.focus_pane(window, pane), None);
                            }
                        }
                    }
                    CyclePreviousBuffer => {
                        let all_buffers = all_buffers(clients, &self.history);
                        let open_buffers = open_buffers(self);

                        if let Some((window, pane, state)) =
                            self.get_focused_mut()
                        {
                            if let Some(buffer) = cycle_previous_buffer(
                                state.buffer.upstream(),
                                all_buffers,
                                &open_buffers,
                            ) {
                                state.buffer = Buffer::from(
                                    data::Buffer::Upstream(buffer),
                                );
                                self.last_changed = Some(Instant::now());
                                return (self.focus_pane(window, pane), None);
                            }
                        }
                    }
                    LeaveBuffer => {
                        if let Some((_, _, state)) = self.get_focused_mut() {
                            if let Some(buffer) =
                                state.buffer.upstream().cloned()
                            {
                                return self.leave_buffer(
                                    clients,
                                    buffer,
                                    config.buffer.mark_as_read.on_buffer_close,
                                );
                            }
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
                                settings.channel.topic.toggle_visibility();
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
                                config,
                                buffer::Internal::FileTransfers,
                            ),
                            None,
                        );
                    }
                    Logs => {
                        return (
                            self.toggle_internal_buffer(
                                config,
                                buffer::Internal::Logs,
                            ),
                            None,
                        );
                    }
                    ThemeEditor => {
                        return (
                            self.toggle_theme_editor(theme, main_window),
                            None,
                        );
                    }
                    Highlight => {
                        return (
                            self.toggle_internal_buffer(
                                config,
                                buffer::Internal::Highlights,
                            ),
                            None,
                        );
                    }
                    ToggleFullscreen => {
                        return (window::toggle_fullscreen(), None);
                    }
                    QuitApplication => return (self.exit(config), None),
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
                        if config.buffer.chathistory.infinite_scroll {
                            if let Some((_, _, state)) = self.get_focused() {
                                if let Some(buffer) = state.buffer.data() {
                                    self.request_older_chathistory(
                                        clients, &buffer,
                                    );
                                }
                            }
                        }

                        return (
                            self.get_focused_mut().map_or_else(
                                Task::none,
                                |(window, id, pane)| {
                                    pane.buffer.scroll_to_start().map(
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
                                    .scroll_to_end()
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

                        if let Some((window, pane, state)) =
                            self.get_focused_mut()
                        {
                            if let Some(buffer) = cycle_next_unread_buffer(
                                state.buffer.upstream(),
                                all_buffers,
                                &open_buffers,
                            ) {
                                state.buffer = Buffer::from(
                                    data::Buffer::Upstream(buffer),
                                );
                                self.last_changed = Some(Instant::now());
                                return (self.focus_pane(window, pane), None);
                            }
                        }
                    }
                    CyclePreviousUnreadBuffer => {
                        let all_buffers =
                            all_buffers_with_has_unread(clients, &self.history);
                        let open_buffers = open_buffers(self);

                        if let Some((window, pane, state)) =
                            self.get_focused_mut()
                        {
                            if let Some(buffer) = cycle_previous_unread_buffer(
                                state.buffer.upstream(),
                                all_buffers,
                                &open_buffers,
                            ) {
                                state.buffer = Buffer::from(
                                    data::Buffer::Upstream(buffer),
                                );
                                self.last_changed = Some(Instant::now());
                                return (self.focus_pane(window, pane), None);
                            }
                        }
                    }
                    MarkAsRead => {
                        if let Some((_, _, pane)) = self.get_focused_mut() {
                            if let Some(kind) = pane
                                .buffer
                                .data()
                                .and_then(history::Kind::from_buffer)
                            {
                                self.mark_as_read(kind, clients);
                            }
                        }
                    }
                }
            }
            Message::FileTransfer(update) => {
                self.file_transfers.update(update);
            }
            Message::SendFileSelected(server, to, path) => {
                if let Some(server_handle) = clients.get_server_handle(&server)
                {
                    if let Some(path) = path {
                        if let Ok(query) = target::Query::parse(
                            to.as_ref(),
                            clients.get_chantypes(&server),
                            clients.get_statusmsg(&server),
                            clients.get_casemapping(&server),
                        ) {
                            if let Some(event) = self.file_transfers.send(
                                file_transfer::SendRequest {
                                    to,
                                    path,
                                    server: server.clone(),
                                    server_handle: server_handle.clone(),
                                },
                                config.proxy.clone(),
                            ) {
                                return (
                                    self.handle_file_transfer_event(
                                        &server, &query, event,
                                    ),
                                    None,
                                );
                            }
                        }
                    }
                }
            }
            Message::CloseContextMenu(window, any_closed) => {
                if !any_closed {
                    if let Some((_, _, state)) = self.get_focused_mut() {
                        if state.buffer.close_picker() {
                            return (Task::none(), None);
                        }
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
            Message::ConfigReloaded(config) => {
                return (Task::none(), Some(Event::ConfigReloaded(config)));
            }
            Message::Client(message) => match message {
                client::Message::ChatHistoryRequest(server, subcommand) => {
                    clients.send_chathistory_request(&server, subcommand);
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
                    );
                }
            },
            Message::LoadPreview((url, Ok(preview))) => {
                debug!("Preview loaded for {url}");
                if let hash_map::Entry::Occupied(mut entry) =
                    self.previews.entry(url)
                {
                    *entry.get_mut() = preview::State::Loaded(preview);
                }
            }
            Message::LoadPreview((url, Err(error))) => {
                error!("Failed to load preview for {url}: {error}");
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
        } else if let Some(editor) = self.theme_editor.as_ref() {
            if editor.window == window {
                return editor.view(theme).map(Message::ThemeEditor);
            }
        }

        column![].into()
    }

    pub fn view<'a>(
        &'a self,
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
                clients,
                &self.history,
                &self.panes,
                self.focus,
                config,
                &self.file_transfers,
                version,
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
                container(Space::new(Length::Fill, Length::Fill))
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
            Copy => selectable_text::selected(Message::SelectedText),
            LeftClick => self.refocus_pane(),
        }
    }

    fn toggle_theme_editor(
        &mut self,
        theme: &mut Theme,
        main_window: &Window,
    ) -> Task<Message> {
        if let Some(editor) = self.theme_editor.take() {
            *theme = theme.selected();
            window::close(editor.window)
        } else {
            let (editor, task) = ThemeEditor::open(main_window);

            self.theme_editor = Some(editor);

            task.then(|_| Task::none())
        }
    }

    fn toggle_internal_buffer(
        &mut self,
        config: &Config,
        buffer: buffer::Internal,
    ) -> Task<Message> {
        let panes = self.panes.clone();

        let open = panes.iter().find_map(|(window_id, pane, state)| {
            (state.buffer.internal().as_ref() == Some(&buffer))
                .then_some((window_id, pane))
        });

        if let Some((window, pane)) = open {
            self.close_pane(window, pane)
        } else {
            self.open_buffer(
                data::Buffer::Internal(buffer),
                config.actions.buffer.local,
                config,
            )
        }
    }

    fn open_buffer(
        &mut self,
        buffer: data::Buffer,
        buffer_action: BufferAction,
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
                    state.buffer = Buffer::from(buffer);
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
                                *p = Pane::new(Buffer::from(buffer));
                            });
                            self.last_changed = Some(Instant::now());

                            return self.focus_pane(self.main_window(), *id);
                        }
                    }
                }

                let pane_to_split = {
                    if self.focus.window == self.main_window() {
                        self.focus.pane
                    } else if let Some(pane) =
                        self.panes.main.panes.keys().last()
                    {
                        *pane
                    } else {
                        log::error!("Didn't find any panes to split");
                        return Task::none();
                    }
                };

                let result = self.panes.main.split(
                    match config.pane.split_axis {
                        config::pane::SplitAxis::Horizontal => {
                            pane_grid::Axis::Horizontal
                        }
                        config::pane::SplitAxis::Vertical => {
                            pane_grid::Axis::Vertical
                        }
                    },
                    pane_to_split,
                    Pane::new(Buffer::from(buffer)),
                );

                if let Some((pane, _)) = result {
                    return self.focus_pane(self.main_window(), pane);
                }

                Task::none()
            }
            BufferAction::NewWindow => {
                get_position(self.main_window()).then(
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
                            ..window::settings()
                        });

                        task.map({
                            let pane = Pane::new(Buffer::from(buffer.clone()));
                            move |id| Message::NewWindow(id, pane.clone())
                        })
                    },
                )
            }
        }
    }

    pub fn leave_buffer(
        &mut self,
        clients: &mut data::client::Map,
        buffer: buffer::Upstream,
        mark_as_read: bool,
    ) -> (Task<Message>, Option<Event>) {
        let open = self.panes.iter().find_map(|(window, pane, state)| {
            (state.buffer.upstream() == Some(&buffer)).then_some((window, pane))
        });

        let mut tasks = vec![];

        // Close pane
        if let Some((window, pane)) = open {
            tasks.push(self.close_pane(window, pane));

            self.last_changed = Some(Instant::now());
        }

        match buffer.clone() {
            buffer::Upstream::Server(server) => {
                (Task::batch(tasks), Some(Event::QuitServer(server)))
            }
            buffer::Upstream::Channel(server, channel) => {
                // Send part & close history file
                let command = command::Irc::Part(channel.to_string(), None);
                let input = data::Input::command(buffer.clone(), command);

                if let Some(encoded) = input.encoded() {
                    clients.send(&buffer, encoded);
                }

                tasks.push(
                    self.history
                        .close(
                            history::Kind::Channel(server, channel),
                            mark_as_read,
                        )
                        .map_or_else(Task::none, |task| {
                            Task::perform(task, Message::History)
                        }),
                );

                (Task::batch(tasks), None)
            }
            buffer::Upstream::Query(server, nick) => {
                tasks.push(
                    self.history
                        .close(history::Kind::Query(server, nick), mark_as_read)
                        .map_or_else(Task::none, |task| {
                            Task::perform(task, Message::History)
                        }),
                );

                // No PART to send, just close history
                (Task::batch(tasks), None)
            }
        }
    }

    pub fn record_message(
        &mut self,
        server: &Server,
        message: data::Message,
    ) -> Task<Message> {
        if let Some(task) = self.history.record_message(server, message) {
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
        if let Some(task) = self.history.record_highlight(message) {
            Task::perform(task, Message::History)
        } else {
            Task::none()
        }
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

        if clients.get_server_supports_chathistory(server) {
            if let Some(target) = upstream.target() {
                if clients.get_chathistory_exhausted(server, &target) {
                    return;
                }

                let message_reference_types = clients
                    .get_server_chathistory_message_reference_types(server);

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

                clients.send_chathistory_request(server, subcommand);
            }
        }
    }

    pub fn broadcast(
        &mut self,
        server: &Server,
        config: &Config,
        sent_time: DateTime<Utc>,
        broadcast: Broadcast,
    ) -> Task<Message> {
        Task::batch(
            self.history
                .broadcast(server, broadcast, config, sent_time)
                .into_iter()
                .map(|task| Task::perform(task, Message::History)),
        )
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

    fn get_focused(&self) -> Option<(window::Id, pane_grid::Pane, &Pane)> {
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
    ) -> Option<(pane_grid::Pane, &mut Pane, &mut history::Manager)> {
        let Focus { window, pane } = self.focus;
        self.panes
            .get_mut(window, pane)
            .map(|state| (pane, state, &mut self.history))
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

    pub fn focus_window(&mut self, window: window::Id) -> Task<Message> {
        let task = if let Some(pane) = self
            .focus_history
            .front()
            .filter(|_| window == self.main_window())
        {
            self.focus_pane(window, *pane)
        } else {
            self.focus_first_pane(window)
        };

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
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Task<Message> {
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

    fn popout_pane(&mut self, config: &Config) -> Task<Message> {
        let Focus { pane, .. } = self.focus;

        self.focus_history = self
            .focus_history
            .clone()
            .into_iter()
            .filter(|p| *p != pane)
            .collect();

        if let Some((pane, _)) = self.panes.main.close(pane) {
            if let Some(buffer) = pane.buffer.data() {
                return self.open_buffer(
                    buffer,
                    BufferAction::NewWindow,
                    config,
                );
            }
        }

        Task::none()
    }

    fn merge_pane(&mut self, config: &Config) -> Task<Message> {
        let Focus { window, pane } = self.focus;

        if let Some(pane) = self
            .panes
            .popout
            .remove(&window)
            .and_then(|panes| panes.get(pane).cloned())
        {
            let task = match pane.buffer.data() {
                Some(buffer) => {
                    self.open_buffer(buffer, BufferAction::NewPane, config)
                }
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

    pub fn track(&mut self, config: &Config) -> Task<Message> {
        let resources = self.panes.resources().collect();

        Task::batch(
            self.history
                .track(resources, config)
                .into_iter()
                .map(|fut| Task::perform(fut, Message::History))
                .collect::<Vec<_>>(),
        )
    }

    pub fn tick(&mut self, now: Instant) -> Task<Message> {
        let history = Task::batch(
            self.history
                .tick(now.into())
                .into_iter()
                .map(|task| Task::perform(task, Message::History))
                .collect::<Vec<_>>(),
        );

        if let Some(last_changed) = self.last_changed {
            if now.duration_since(last_changed) >= SAVE_AFTER {
                let dashboard = data::Dashboard::from(&*self);

                self.last_changed = None;

                return Task::batch(vec![
                    Task::perform(dashboard.save(), Message::DashboardSaved),
                    history,
                ]);
            }
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
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
        request: file_transfer::ReceiveRequest,
        config: &Config,
    ) -> Option<Task<Message>> {
        let event = self
            .file_transfers
            .receive(request.clone(), config.proxy.as_ref())?;

        self.notifications.notify(
            &config.notifications,
            &Notification::FileTransferRequest(request.from.clone()),
            server,
        );

        let query = target::Query::parse(
            request.from.as_ref(),
            chantypes,
            statusmsg,
            casemapping,
        )
        .ok()?;

        Some(self.handle_file_transfer_event(server, &query, event))
    }

    pub fn handle_file_transfer_event(
        &mut self,
        server: &Server,
        query: &target::Query,
        event: file_transfer::manager::Event,
    ) -> Task<Message> {
        let mut tasks = vec![];

        match event {
            file_transfer::manager::Event::NewTransfer(transfer, task) => {
                match transfer.direction {
                    file_transfer::Direction::Received => {
                        tasks.push(self.record_message(
                            server,
                            data::Message::file_transfer_request_received(
                                &transfer.remote_user,
                                query,
                                &transfer.filename,
                            ),
                        ));
                    }
                    file_transfer::Direction::Sent => {
                        tasks.push(self.record_message(
                            server,
                            data::Message::file_transfer_request_sent(
                                &transfer.remote_user,
                                query,
                                &transfer.filename,
                            ),
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

        fn configuration(pane: data::Pane) -> Configuration<Pane> {
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
                        a: Box::new(configuration(*a)),
                        b: Box::new(configuration(*b)),
                    }
                }
                data::Pane::Buffer { buffer } => {
                    Configuration::Pane(Pane::new(Buffer::from(buffer)))
                }
                data::Pane::Empty => {
                    Configuration::Pane(Pane::new(Buffer::empty()))
                }
            }
        }

        let panes = Panes {
            main_window: main_window.id,
            main: pane_grid::State::with_configuration(configuration(
                data.pane,
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
            file_transfers: file_transfer::Manager::new(
                config.file_transfer.clone(),
            ),
            theme_editor: None,
            notifications: notification::Notifications::new(),
            previews: preview::Collection::default(),
            buffer_settings: data.buffer_settings.clone(),
        };

        let mut tasks = vec![];

        for pane in data.popout_panes {
            // Popouts are only a single pane
            let Configuration::Pane(pane) = configuration(pane) else {
                continue;
            };

            if let Some(buffer) = pane.buffer.data() {
                tasks.push(dashboard.open_buffer(
                    buffer,
                    BufferAction::NewWindow,
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
                    return self.focus_window(id);
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
        colors: theme::Colors,
        main_window: &Window,
        theme: &mut Theme,
    ) -> Task<Message> {
        *theme = theme.preview(data::Theme::new("Custom Theme".into(), colors));

        if let Some(editor) = &self.theme_editor {
            window::gain_focus(editor.window)
        } else {
            let (editor, task) = ThemeEditor::open(main_window);

            self.theme_editor = Some(editor);

            task.then(|_| Task::none())
        }
    }

    pub fn exit(&mut self, config: &Config) -> Task<Message> {
        let history = self.history.exit(
            config.buffer.mark_as_read.on_application_exit,
            config.buffer.mark_as_read.on_buffer_close
                || config.buffer.mark_as_read.on_application_exit,
        );
        let last_changed = self.last_changed.take();
        let dashboard = data::Dashboard::from(&*self);

        Task::perform(
            async move {
                if last_changed.is_some() {
                    match dashboard.save().await {
                        Ok(()) => {
                            log::info!("dashboard saved");
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
        if !clients
            .get_channels(&server)
            .iter()
            .any(|joined| channel == *joined)
        {
            clients.join(&server, slice::from_ref(&channel));
        }

        self.open_buffer(data::Buffer::Upstream(buffer), buffer_action, config)
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

                self.open_buffer(buffer.clone(), buffer_action, config)
            }
        }
    }

    fn main_window(&self) -> window::Id {
        self.panes.main_window
    }

    fn mark_as_read(
        &mut self,
        kind: history::Kind,
        clients: &mut data::client::Map,
    ) {
        let read_marker = self.history.mark_as_read(&kind);

        if let (Some(server), Some(target), Some(read_marker)) =
            (kind.server(), kind.target(), read_marker)
        {
            let _ = clients.send_markread(server, target, read_marker);
        }
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
                .chain(clients.get_channels(server).iter().map(|channel| {
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
            .chain(clients.get_channels(server).iter().map(|channel| {
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

    next_after().or_else(|| next_before().or(current)).cloned()
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
        .or_else(|| previous_after().or(current))
        .cloned()
}
