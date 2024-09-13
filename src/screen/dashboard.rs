use chrono::{DateTime, Utc};
use data::environment::RELEASE_WEBSITE;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use data::history::manager::Broadcast;
use data::isupport::{ChatHistorySubcommand, MessageReference};
use data::user::Nick;
use data::{client, environment, file_transfer, history, isupport, Config, Server, User, Version};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{column, container, row, Space};
use iced::{clipboard, Length, Task, Vector};

use self::command_bar::CommandBar;
use self::pane::Pane;
use self::sidebar::Sidebar;
use self::theme_editor::ThemeEditor;
use crate::buffer::file_transfers::FileTransfers;
use crate::buffer::{self, Buffer};
use crate::widget::{
    anchored_overlay, context_menu, selectable_text, shortcut, Column, Element, Row,
};
use crate::window::Window;
use crate::{event, notification, theme, window, Theme};

mod command_bar;
pub mod pane;
pub mod sidebar;
mod theme_editor;

const SAVE_AFTER: Duration = Duration::from_secs(3);

pub struct Dashboard {
    panes: Panes,
    focus: Option<(window::Id, pane_grid::Pane)>,
    side_menu: Sidebar,
    history: history::Manager,
    last_changed: Option<Instant>,
    command_bar: Option<CommandBar>,
    file_transfers: file_transfer::Manager,
    theme_editor: Option<ThemeEditor>,
}

#[derive(Debug)]
pub enum Message {
    Pane(window::Id, pane::Message),
    Sidebar(sidebar::Message),
    SelectedText(Vec<(f32, String)>),
    History(history::manager::Message),
    DashboardSaved(Result<(), data::dashboard::Error>),
    CloseHistory,
    Task(command_bar::Message),
    Shortcut(shortcut::Command),
    FileTransfer(file_transfer::task::Update),
    SendFileSelected(Server, Nick, Option<PathBuf>),
    CloseContextMenu(bool),
    ThemeEditor(theme_editor::Message),
}

#[derive(Debug)]
pub enum Event {
    ReloadConfiguration,
    ReloadThemes,
    QuitServer(Server),
}

impl Dashboard {
    pub fn empty(config: &Config) -> (Self, Task<Message>) {
        let (main_panes, _) = pane_grid::State::new(Pane::new(Buffer::Empty, config));

        let mut dashboard = Dashboard {
            panes: Panes {
                main: main_panes,
                popout: HashMap::new(),
            },
            focus: None,
            side_menu: Sidebar::new(),
            history: history::Manager::default(),
            last_changed: None,
            command_bar: None,
            file_transfers: file_transfer::Manager::new(config.file_transfer.clone()),
            theme_editor: None,
        };

        let command = dashboard.track();

        (dashboard, command)
    }

    pub fn restore(
        dashboard: data::Dashboard,
        config: &Config,
        main_window: &Window,
    ) -> (Self, Task<Message>) {
        let (mut dashboard, task) = Dashboard::from_data(dashboard, config, main_window);

        let command = if let Some((pane, _)) = dashboard.panes.main.panes.iter().next() {
            Task::batch(vec![
                dashboard.focus_pane(main_window, main_window.id, *pane),
                dashboard.track(),
            ])
        } else {
            dashboard.track()
        };

        (dashboard, Task::batch(vec![task, command]))
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
                        return (self.focus_pane(main_window, window, pane), None);
                    }
                    pane::Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                        // Pane grid interactions only enabled for main window panegrid
                        self.panes.main.resize(split, ratio);
                        self.last_changed = Some(Instant::now());
                    }
                    pane::Message::PaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                        // Pane grid interactions only enabled for main window panegrid
                        self.panes.main.drop(pane, target);
                        self.last_changed = Some(Instant::now());
                    }
                    pane::Message::PaneDragged(_) => {}
                    pane::Message::ClosePane => {
                        if let Some((window, pane)) = self.focus.take() {
                            return (self.close_pane(main_window, window, pane), None);
                        }
                    }
                    pane::Message::SplitPane(axis) => {
                        return (self.split_pane(axis, config, main_window), None);
                    }
                    pane::Message::Buffer(id, message) => {
                        if let Some(pane) = self.panes.get_mut(main_window.id, window, id) {
                            let (command, event) = pane.buffer.update(
                                message,
                                clients,
                                &mut self.history,
                                &mut self.file_transfers,
                                config,
                            );

                            if let Some(buffer::Event::UserContext(event)) = event {
                                match event {
                                    buffer::user_context::Event::ToggleAccessLevel(nick, mode) => {
                                        let Some(buffer) = pane.buffer.data() else {
                                            return (Task::none(), None);
                                        };

                                        let Some(target) = buffer.target() else {
                                            return (Task::none(), None);
                                        };

                                        let command = data::Command::Mode(
                                            target,
                                            Some(mode),
                                            Some(vec![nick.to_string()]),
                                        );
                                        let input = data::Input::command(buffer.clone(), command);

                                        if let Some(encoded) = input.encoded() {
                                            clients.send(input.buffer(), encoded);
                                        }
                                    }
                                    buffer::user_context::Event::SendWhois(nick) => {
                                        if let Some(buffer) = pane.buffer.data() {
                                            let command =
                                                data::Command::Whois(None, nick.to_string());

                                            let input =
                                                data::Input::command(buffer.clone(), command);

                                            if let Some(encoded) = input.encoded() {
                                                clients.send(input.buffer(), encoded);
                                            }

                                            if let Some(nick) = clients.nickname(buffer.server()) {
                                                let mut user = nick.to_owned().into();

                                                // Resolve our attributes if sending this message in a channel
                                                if let data::Buffer::Channel(server, channel) =
                                                    &buffer
                                                {
                                                    if let Some(user_with_attributes) = clients
                                                        .resolve_user_attributes(
                                                            server, channel, &user,
                                                        )
                                                    {
                                                        user = user_with_attributes.clone();
                                                    }
                                                }

                                                if let Some(messages) = input.messages(user) {
                                                    for message in messages {
                                                        self.history.record_message(
                                                            input.server(),
                                                            message,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    buffer::user_context::Event::OpenQuery(nick) => {
                                        if let Some(data) = pane.buffer.data() {
                                            let buffer =
                                                data::Buffer::Query(data.server().clone(), nick);
                                            return (
                                                self.open_buffer(
                                                    buffer,
                                                    config.buffer.clone().into(),
                                                    main_window,
                                                ),
                                                None,
                                            );
                                        }
                                    }
                                    buffer::user_context::Event::SingleClick(nick) => {
                                        let Some((_, pane, history)) =
                                            self.get_focused_with_history_mut(main_window)
                                        else {
                                            return (Task::none(), None);
                                        };

                                        return (
                                            pane.buffer.insert_user_to_input(nick, history).map(
                                                move |message| {
                                                    Message::Pane(
                                                        window,
                                                        pane::Message::Buffer(id, message),
                                                    )
                                                },
                                            ),
                                            None,
                                        );
                                    }
                                    buffer::user_context::Event::SendFile(nick) => {
                                        if let Some(buffer) = pane.buffer.data() {
                                            let server = buffer.server().clone();
                                            let starting_directory =
                                                config.file_transfer.save_directory.clone();

                                            return (
                                                Task::perform(
                                                    async move {
                                                        rfd::AsyncFileDialog::new()
                                                            .set_directory(starting_directory)
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
                                                None,
                                            );
                                        }
                                    }
                                }
                            }
                            Some(buffer::Event::ScrolledToTop) => {
                                if config
                                    .buffer
                                    .chathistory
                                    .request_older_messages_at_scroll_top
                                {
                                    if let Some(data::Buffer::Channel(server, channel)) =
                                        &pane.buffer.data()
                                    {
                                        if clients.get_server_supports_chathistory(server) {
                                            let message_reference_type = clients
                                                .get_server_chathistory_message_reference_type(
                                                    server,
                                                );

                                            let oldest_message_reference = self
                                                .get_oldest_message_reference(
                                                    server,
                                                    channel.clone(),
                                                    message_reference_type,
                                                );

                                            clients.send_channel_chathistory_request(
                                                ChatHistorySubcommand::Before,
                                                server,
                                                channel.as_str(),
                                                oldest_message_reference,
                                            );
                                        }
                                    }
                                }
                            }
                            Some(buffer::Event::ChatHistoryBeforeRequest) => {
                                if let Some(data::Buffer::Channel(server, channel)) =
                                    &pane.buffer.data()
                                {
                                    if clients.get_server_supports_chathistory(server) {
                                        let message_reference_type = clients
                                            .get_server_chathistory_message_reference_type(server);

                                        let oldest_message_reference = self
                                            .get_oldest_message_reference(
                                                server,
                                                channel.clone(),
                                                message_reference_type,
                                            );

                                        clients.send_channel_chathistory_request(
                                            ChatHistorySubcommand::Before,
                                            server,
                                            channel.as_str(),
                                            oldest_message_reference,
                                        );
                                    }
                                }
                            }

                            return (
                                command.map(move |message| {
                                    Message::Pane(window, pane::Message::Buffer(id, message))
                                }),
                                None,
                            );
                        }
                    }
                    pane::Message::ToggleShowUserList => {
                        if let Some((_, _, pane)) = self.get_focused_mut(main_window) {
                            pane.update_settings(|settings| {
                                settings.channel.nicklist.toggle_visibility()
                            });
                            self.last_changed = Some(Instant::now());
                        }
                    }
                    pane::Message::ToggleShowTopic => {
                        if let Some((_, _, pane)) = self.get_focused_mut(main_window) {
                            pane.update_settings(|settings| {
                                settings.channel.topic.toggle_visibility()
                            });
                            self.last_changed = Some(Instant::now());
                        }
                    }
                    pane::Message::MaximizePane => self.maximize_pane(),
                    pane::Message::Popout => return (self.popout_pane(main_window), None),
                    pane::Message::Merge => return (self.merge_pane(config, main_window), None),
                }
            }
            Message::Sidebar(message) => {
                let event = self.side_menu.update(message);

                match event {
                    sidebar::Event::Open(kind) => {
                        return (
                            self.open_buffer(kind, config.buffer.clone().into(), main_window),
                            None,
                        );
                    }
                    sidebar::Event::Popout(buffer) => {
                        return (
                            self.open_popout_window(
                                main_window,
                                Pane::new(Buffer::from(buffer), config),
                            ),
                            None,
                        );
                    }
                    sidebar::Event::Focus(window, pane) => {
                        return (self.focus_pane(main_window, window, pane), None);
                    }
                    sidebar::Event::Replace(window, kind, pane) => {
                        if let Some(state) = self.panes.get_mut(main_window.id, window, pane) {
                            state.buffer = Buffer::from(kind);
                            self.last_changed = Some(Instant::now());
                            self.focus = None;
                            return (
                                Task::batch(vec![
                                    self.reset_pane(main_window, window, pane),
                                    self.focus_pane(main_window, window, pane),
                                ]),
                                None,
                            );
                        }
                    }
                    sidebar::Event::Close(window, pane) => {
                        if self.focus == Some((window, pane)) {
                            self.focus = None;
                        }

                        return (self.close_pane(main_window, window, pane), None);
                    }
                    sidebar::Event::Swap(from_window, from_pane, to_window, to_pane) => {
                        self.last_changed = Some(Instant::now());

                        if from_window == main_window.id && to_window == main_window.id {
                            self.panes.main.swap(from_pane, to_pane);

                            return (self.focus_pane(main_window, from_window, from_pane), None);
                        } else if let Some((from_state, to_state)) = self
                            .panes
                            .get(main_window.id, from_window, from_pane)
                            .cloned()
                            .zip(self.panes.get(main_window.id, to_window, to_pane).cloned())
                        {
                            if let Some(state) =
                                self.panes.get_mut(main_window.id, from_window, from_pane)
                            {
                                *state = to_state;
                            }
                            if let Some(state) =
                                self.panes.get_mut(main_window.id, to_window, to_pane)
                            {
                                *state = from_state;
                            }
                        }
                    }
                    sidebar::Event::Leave(buffer) => {
                        return self.leave_buffer(main_window, clients, buffer);
                    }
                    sidebar::Event::ToggleFileTransfers => {
                        return (self.toggle_file_transfers(config, main_window), None);
                    }
                    sidebar::Event::ToggleCommandBar => {
                        return (
                            self.toggle_command_bar(
                                &closed_buffers(self, main_window.id, clients),
                                version,
                                config,
                                theme,
                                main_window,
                            ),
                            None,
                        );
                    }
                    sidebar::Event::ReloadConfigFile => {
                        return (Task::none(), Some(Event::ReloadConfiguration));
                    }
                    sidebar::Event::OpenReleaseWebsite => {
                        let _ = open::that_detached(RELEASE_WEBSITE);
                        return (Task::none(), None);
                    }
                    sidebar::Event::ToggleThemeEditor => {
                        if let Some(editor) = self.theme_editor.take() {
                            *theme = theme.selected();
                            return (window::close(editor.window), None);
                        } else {
                            let (editor, task) = ThemeEditor::open(main_window);

                            self.theme_editor = Some(editor);

                            return (task.then(|_| Task::none()), None);
                        }
                    }
                }
            }
            Message::SelectedText(contents) => {
                let mut last_y = None;
                let contents = contents
                    .into_iter()
                    .fold(String::new(), |acc, (y, content)| {
                        if let Some(_y) = last_y {
                            let new_line = if y == _y { "" } else { "\n" };
                            last_y = Some(y);

                            format!("{acc}{new_line}{content}")
                        } else {
                            last_y = Some(y);

                            content
                        }
                    });

                if !contents.is_empty() {
                    return (clipboard::write(contents), None);
                }
            }
            Message::History(message) => {
                self.history.update(message);
            }
            Message::DashboardSaved(Ok(_)) => {
                log::info!("dashboard saved");
            }
            Message::DashboardSaved(Err(error)) => {
                log::warn!("error saving dashboard: {error}");
            }
            Message::CloseHistory => {}
            Message::Task(message) => {
                let Some(command_bar) = &mut self.command_bar else {
                    return (Task::none(), None);
                };

                match command_bar.update(message) {
                    Some(command_bar::Event::ThemePreview(preview)) => match preview {
                        Some(preview) => *theme = theme.preview(preview),
                        None => *theme = theme.selected(),
                    },
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
                                command_bar::Buffer::New => (
                                    self.new_pane(pane_grid::Axis::Horizontal, config, main_window),
                                    None,
                                ),
                                command_bar::Buffer::Close => {
                                    if let Some((window, pane)) = self.focus {
                                        (self.close_pane(main_window, window, pane), None)
                                    } else {
                                        (Task::none(), None)
                                    }
                                }
                                command_bar::Buffer::Replace(buffer) => {
                                    let mut commands = vec![];

                                    if let Some((window, pane)) = self.focus.take() {
                                        if let Some(state) =
                                            self.panes.get_mut(main_window.id, window, pane)
                                        {
                                            state.buffer = Buffer::from(buffer);
                                            self.last_changed = Some(Instant::now());

                                            commands.extend(vec![
                                                self.reset_pane(main_window, window, pane),
                                                self.focus_pane(main_window, window, pane),
                                            ]);
                                        }
                                    }

                                    (Task::batch(commands), None)
                                }
                                command_bar::Buffer::Popout => {
                                    (self.popout_pane(main_window), None)
                                }
                                command_bar::Buffer::Merge => {
                                    (self.merge_pane(config, main_window), None)
                                }
                                command_bar::Buffer::ToggleFileTransfers => {
                                    (self.toggle_file_transfers(config, main_window), None)
                                }
                            },
                            command_bar::Command::Configuration(command) => match command {
                                command_bar::Configuration::OpenDirectory => {
                                    let _ = open::that_detached(Config::config_dir());
                                    (Task::none(), None)
                                }
                                command_bar::Configuration::OpenWebsite => {
                                    let _ = open::that_detached(environment::WIKI_WEBSITE);
                                    (Task::none(), None)
                                }
                                command_bar::Configuration::Reload => {
                                    (Task::none(), Some(Event::ReloadConfiguration))
                                }
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
                            },
                        };

                        return (
                            Task::batch(vec![
                                command,
                                self.toggle_command_bar(
                                    &closed_buffers(self, main_window.id, clients),
                                    version,
                                    config,
                                    theme,
                                    main_window,
                                ),
                            ]),
                            event,
                        );
                    }
                    Some(command_bar::Event::Unfocused) => {
                        return (
                            self.toggle_command_bar(
                                &closed_buffers(self, main_window.id, clients),
                                version,
                                config,
                                theme,
                                main_window,
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
                    if let Some((window, pane)) = self.focus.as_ref() {
                        if *window == main_window.id {
                            if let Some(adjacent) = self.panes.main.adjacent(*pane, direction) {
                                return self.focus_pane(main_window, *window, adjacent);
                            }
                        }
                    } else if let Some((pane, _)) = self.panes.main.panes.iter().next() {
                        return self.focus_pane(main_window, main_window.id, *pane);
                    }

                    Task::none()
                };

                match shortcut {
                    MoveUp => return (move_focus(pane_grid::Direction::Up), None),
                    MoveDown => return (move_focus(pane_grid::Direction::Down), None),
                    MoveLeft => return (move_focus(pane_grid::Direction::Left), None),
                    MoveRight => return (move_focus(pane_grid::Direction::Right), None),
                    CloseBuffer => {
                        if let Some((window, pane)) = self.focus {
                            return (self.close_pane(main_window, window, pane), None);
                        }
                    }
                    MaximizeBuffer => {
                        if let Some((window, pane)) = self.focus.as_ref() {
                            // Only main window has >1 pane to maximize
                            if *window == main_window.id {
                                self.panes.main.maximize(*pane);
                            }
                        }
                    }
                    RestoreBuffer => {
                        self.panes.main.restore();
                    }
                    CycleNextBuffer => {
                        let all_buffers = all_buffers(clients, &self.history);
                        let open_buffers = open_buffers(self, main_window.id);

                        if let Some((window, pane, state)) = self.get_focused_mut(main_window) {
                            if let Some(buffer) = cycle_next_buffer(
                                state.buffer.data().as_ref(),
                                all_buffers,
                                &open_buffers,
                            ) {
                                state.buffer = Buffer::from(buffer);
                                self.focus = None;
                                return (self.focus_pane(main_window, window, pane), None);
                            }
                        }
                    }
                    CyclePreviousBuffer => {
                        let all_buffers = all_buffers(clients, &self.history);
                        let open_buffers = open_buffers(self, main_window.id);

                        if let Some((window, pane, state)) = self.get_focused_mut(main_window) {
                            if let Some(buffer) = cycle_previous_buffer(
                                state.buffer.data().as_ref(),
                                all_buffers,
                                &open_buffers,
                            ) {
                                state.buffer = Buffer::from(buffer);
                                self.focus = None;
                                return (self.focus_pane(main_window, window, pane), None);
                            }
                        }
                    }
                    LeaveBuffer => {
                        if let Some((_, _, state)) = self.get_focused_mut(main_window) {
                            if let Some(buffer) = state.buffer.data() {
                                return self.leave_buffer(main_window, clients, buffer);
                            }
                        }
                    }
                    ToggleNicklist => {
                        if let Some((_, _, pane)) = self.get_focused_mut(main_window) {
                            pane.update_settings(|settings| {
                                settings.channel.nicklist.enabled =
                                    !settings.channel.nicklist.enabled
                            });
                        }
                    }
                    ToggleSidebar => {
                        self.side_menu.toggle_visibility();
                    }
                    CommandBar => {
                        return (
                            self.toggle_command_bar(
                                &closed_buffers(self, main_window.id, clients),
                                version,
                                config,
                                theme,
                                main_window,
                            ),
                            None,
                        );
                    }
                    ReloadConfiguration => return (Task::none(), Some(Event::ReloadConfiguration)),
                }
            }
            Message::FileTransfer(update) => {
                self.file_transfers.update(update);
            }
            Message::SendFileSelected(server, to, path) => {
                if let Some(server_handle) = clients.get_server_handle(&server) {
                    if let Some(path) = path {
                        if let Some(event) = self.file_transfers.send(
                            file_transfer::SendRequest {
                                to,
                                path,
                                server: server.clone(),
                                server_handle: server_handle.clone(),
                            },
                            config.proxy.clone(),
                        ) {
                            return (self.handle_file_transfer_event(&server, event), None);
                        }
                    }
                }
            }
            Message::CloseContextMenu(any_closed) => {
                if !any_closed {
                    if self.is_pane_maximized() {
                        self.panes.main.restore();
                    } else {
                        self.focus = None;
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
        }

        (Task::none(), None)
    }

    pub fn view_window<'a>(
        &'a self,
        window: window::Id,
        clients: &'a client::Map,
        config: &'a Config,
        theme: &'a Theme,
        main_window: &'a Window,
    ) -> Element<'a, Message> {
        if let Some(state) = self.panes.popout.get(&window) {
            let content = container(
                PaneGrid::new(state, |id, pane, _maximized| {
                    let is_focused = self.focus == Some((window, id));
                    pane.view(
                        id,
                        window,
                        1,
                        is_focused,
                        false,
                        clients,
                        &self.file_transfers,
                        &self.history,
                        &self.side_menu,
                        config,
                        theme,
                        main_window,
                    )
                })
                .spacing(4)
                .on_click(pane::Message::PaneClicked),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8);

            return Element::new(content).map(move |message| Message::Pane(window, message));
        } else if let Some(editor) = self.theme_editor.as_ref() {
            if editor.window == window {
                return editor.view(theme).map(Message::ThemeEditor);
            }
        }

        column![].into()
    }

    pub fn view<'a>(
        &'a self,
        now: Instant,
        clients: &'a client::Map,
        version: &'a Version,
        config: &'a Config,
        theme: &'a Theme,
        main_window: &'a Window,
    ) -> Element<'a, Message> {
        let focus = self.focus;

        let pane_grid: Element<_> = PaneGrid::new(&self.panes.main, |id, pane, maximized| {
            let is_focused = focus == Some((main_window.id, id));
            let panes = self.panes.main.panes.len();
            pane.view(
                id,
                main_window.id,
                panes,
                is_focused,
                maximized,
                clients,
                &self.file_transfers,
                &self.history,
                &self.side_menu,
                config,
                theme,
                main_window,
            )
        })
        .on_click(pane::Message::PaneClicked)
        .on_resize(6, pane::Message::PaneResized)
        .on_drag(pane::Message::PaneDragged)
        .spacing(4)
        .into();

        let pane_grid =
            container(pane_grid.map(move |message| Message::Pane(main_window.id, message)))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(8);

        let side_menu = self
            .side_menu
            .view(
                now,
                clients,
                &self.history,
                &self.panes,
                self.focus,
                config.sidebar,
                config.tooltips,
                &self.file_transfers,
                version,
                self.theme_editor.is_some(),
                main_window.id,
            )
            .map(|e| e.map(Message::Sidebar));

        let content = match config.sidebar.position {
            data::config::sidebar::Position::Left | data::config::sidebar::Position::Top => {
                vec![side_menu.unwrap_or_else(|| row![].into()), pane_grid.into()]
            }
            data::config::sidebar::Position::Right | data::config::sidebar::Position::Bottom => {
                vec![pane_grid.into(), side_menu.unwrap_or_else(|| row![].into())]
            }
        };

        let base: Element<Message> = if config.sidebar.position.is_horizontal() {
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
                        main_window.id,
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
        event: event::Event,
        clients: &mut data::client::Map,
        version: &Version,
        config: &Config,
        theme: &mut Theme,
        main_window: &Window,
    ) -> Task<Message> {
        use event::Event::*;

        match event {
            Escape => {
                // Order of operations
                //
                // - Close command bar
                // - Close context menu
                // - Restore maximized pane
                // - Unfocus
                if self.command_bar.is_some() {
                    self.toggle_command_bar(
                        &closed_buffers(self, main_window.id, clients),
                        version,
                        config,
                        theme,
                        main_window,
                    )
                } else {
                    context_menu::close(Message::CloseContextMenu)
                }
            }
            Copy => selectable_text::selected(Message::SelectedText),
            Home => {
                if config
                    .buffer
                    .chathistory
                    .request_older_messages_at_scroll_top
                {
                    if let Some((_, pane)) = self.get_focused_mut() {
                        if let Some(data::Buffer::Channel(server, channel)) = pane.buffer.data() {
                            if clients.get_server_supports_chathistory(&server) {
                                let message_reference_type =
                                    clients.get_server_chathistory_message_reference_type(&server);

                                let oldest_message_reference = self.get_oldest_message_reference(
                                    &server,
                                    channel.clone(),
                                    message_reference_type,
                                );

                                clients.send_channel_chathistory_request(
                                    ChatHistorySubcommand::Before,
                                    &server,
                                    channel.as_str(),
                                    oldest_message_reference,
                                );
                            }
                        }
                    }
                }

                self.get_focused_mut()
                    .map(|(id, pane)| {
                        pane.buffer
                            .scroll_to_start()
                            .map(move |message| Message::Pane(pane::Message::Buffer(id, message)))
                    })
                    .unwrap_or_else(Command::none)
            }
            End => self
                .get_focused_mut(main_window)
                .map(|(window, pane, state)| {
                    state.buffer.scroll_to_end().map(move |message| {
                        Message::Pane(window, pane::Message::Buffer(pane, message))
                    })
                })
                .unwrap_or_else(Task::none),
        }
    }

    // TODO: Perhaps rewrite this, i just did this quickly.
    fn toggle_file_transfers(&mut self, config: &Config, main_window: &Window) -> Task<Message> {
        let panes = self.panes.clone();

        // If file transfers already is open, we close it.
        for (window, id, pane) in panes.iter(main_window.id) {
            if matches!(pane.buffer, Buffer::FileTransfers(_)) {
                return self.close_pane(main_window, window, id);
            }
        }

        // If we only have one pane, and its empty, we replace it.
        if self.panes.len() == 1 {
            for (id, pane) in panes.main.iter() {
                if let Buffer::Empty = &pane.buffer {
                    self.panes.main.panes.entry(*id).and_modify(|p| {
                        *p = Pane::new(Buffer::FileTransfers(FileTransfers::new()), config)
                    });
                    self.last_changed = Some(Instant::now());

                    return self.focus_pane(main_window, main_window.id, *id);
                }
            }
        }

        let mut commands = vec![];
        let _ = self.new_pane(pane_grid::Axis::Vertical, config, main_window);

        if let Some((window, pane)) = self.focus.take() {
            if let Some(state) = self.panes.get_mut(main_window.id, window, pane) {
                state.buffer = Buffer::FileTransfers(FileTransfers::new());
                self.last_changed = Some(Instant::now());

                commands.extend(vec![
                    self.reset_pane(main_window, window, pane),
                    self.focus_pane(main_window, window, pane),
                ]);
            }
        }

        Task::batch(commands)
    }

    fn open_buffer(
        &mut self,
        kind: data::Buffer,
        settings: buffer::Settings,
        main_window: &Window,
    ) -> Task<Message> {
        let panes = self.panes.clone();

        // If channel already is open, we focus it.
        for (window, id, pane) in panes.iter(main_window.id) {
            if pane.buffer.data().as_ref() == Some(&kind) {
                self.focus = Some((window, id));

                return self.focus_pane(main_window, window, id);
            }
        }

        // If we only have one pane, and its empty, we replace it.
        if self.panes.len() == 1 {
            for (id, pane) in panes.main.iter() {
                if matches!(pane.buffer, Buffer::Empty) {
                    self.panes
                        .main
                        .panes
                        .entry(*id)
                        .and_modify(|p| *p = Pane::with_settings(Buffer::from(kind), settings));
                    self.last_changed = Some(Instant::now());

                    return self.focus_pane(main_window, main_window.id, *id);
                }
            }
        }

        // Default split could be a config option.
        let axis = pane_grid::Axis::Horizontal;
        let pane_to_split = {
            if let Some((_, pane)) = self.focus.filter(|(window, _)| *window == main_window.id) {
                pane
            } else if let Some(pane) = self.panes.main.panes.keys().last() {
                *pane
            } else {
                log::error!("Didn't find any panes");
                return Task::none();
            }
        };

        let result = self.panes.main.split(
            axis,
            pane_to_split,
            Pane::with_settings(Buffer::from(kind), settings),
        );
        self.last_changed = Some(Instant::now());

        if let Some((pane, _)) = result {
            return self.focus_pane(main_window, main_window.id, pane);
        }

        Task::none()
    }

    pub fn leave_buffer(
        &mut self,
        main_window: &Window,
        clients: &mut data::client::Map,
        buffer: data::Buffer,
    ) -> (Task<Message>, Option<Event>) {
        let open = self
            .panes
            .iter(main_window.id)
            .find_map(|(window, pane, state)| {
                (state.buffer.data().as_ref() == Some(&buffer)).then_some((window, pane))
            });

        let mut tasks = vec![];

        // Close pane
        if let Some((window, pane)) = open {
            if self.focus == Some((window, pane)) {
                self.focus = None;
            }

            tasks.push(self.close_pane(main_window, window, pane));

            self.last_changed = Some(Instant::now());
        }

        match buffer.clone() {
            data::Buffer::Server(server) => (Task::batch(tasks), Some(Event::QuitServer(server))),
            data::Buffer::Channel(server, channel) => {
                // Send part & close history file
                let command = data::Command::Part(channel.clone(), None);
                let input = data::Input::command(buffer.clone(), command);

                if let Some(encoded) = input.encoded() {
                    clients.send(&buffer, encoded);
                }

                tasks.push(
                    self.history
                        .close(server, history::Kind::Channel(channel))
                        .map(|task| Task::perform(task, |_| Message::CloseHistory))
                        .unwrap_or_else(Task::none),
                );

                (Task::batch(tasks), None)
            }
            data::Buffer::Query(server, nick) => {
                tasks.push(
                    self.history
                        .close(server, history::Kind::Query(nick))
                        .map(|task| Task::perform(task, |_| Message::CloseHistory))
                        .unwrap_or_else(Task::none),
                );

                // No PART to send, just close history
                (Task::batch(tasks), None)
            }
        }
    }

    pub fn record_message(&mut self, server: &Server, message: data::Message) {
        self.history.record_message(server, message);
    }

    pub fn record_chathistory_message(
        &mut self,
        server: &Server,
        message: data::Message,
        subcommand: ChatHistorySubcommand,
        message_reference: MessageReference,
    ) {
        self.history
            .record_chathistory_message(server, message, subcommand, message_reference);
    }

    pub fn is_open(&self, server: Server, channel: String) -> bool {
        self.panes.iter().any(|(_, pane)| {
            pane.buffer.data() == Some(data::Buffer::Channel(server.clone(), channel.clone()))
        })
    }

    pub fn load_history_now(&mut self, server: Server, channel: String) {
        self.history
            .load_now(server, history::Kind::Channel(channel.clone()));
    }

    pub fn make_history_partial_now(
        &mut self,
        server: Server,
        channel: String,
        message_reference: Option<isupport::MessageReference>,
    ) {
        self.history.make_partial_now(
            server,
            history::Kind::Channel(channel.clone()),
            message_reference,
        );
    }

    pub fn get_latest_message_reference(
        &self,
        server: &Server,
        channel: String,
        message_reference_type: isupport::MessageReferenceType,
        join_server_time: DateTime<Utc>,
    ) -> MessageReference {
        let latest_message = match message_reference_type {
            isupport::MessageReferenceType::MessageId => self
                .history
                .get_latest_message(
                    server,
                    &history::Kind::Channel(channel.clone()),
                    isupport::MessageReferenceType::MessageId,
                    join_server_time,
                )
                .or(self.history.get_latest_message(
                    server,
                    &history::Kind::Channel(channel.clone()),
                    isupport::MessageReferenceType::Timestamp,
                    join_server_time,
                )),
            isupport::MessageReferenceType::Timestamp => self.history.get_latest_message(
                server,
                &history::Kind::Channel(channel.clone()),
                isupport::MessageReferenceType::Timestamp,
                join_server_time,
            ),
        };

        if let Some(latest_message) = latest_message {
            if matches!(
                message_reference_type,
                isupport::MessageReferenceType::MessageId
            ) {
                if let Some(id) = &latest_message.id {
                    return MessageReference::MessageId(id.clone());
                }
            }

            MessageReference::Timestamp(
                latest_message.server_time,
                latest_message.id.clone().unwrap_or(":".to_string()),
            )
        } else {
            MessageReference::None
        }
    }

    pub fn get_oldest_message_reference(
        &self,
        server: &Server,
        channel: String,
        message_reference_type: isupport::MessageReferenceType,
    ) -> MessageReference {
        let oldest_message = match message_reference_type {
            isupport::MessageReferenceType::MessageId => self
                .history
                .get_oldest_message(
                    server,
                    &history::Kind::Channel(channel.clone()),
                    isupport::MessageReferenceType::MessageId,
                )
                .or(self.history.get_oldest_message(
                    server,
                    &history::Kind::Channel(channel.clone()),
                    isupport::MessageReferenceType::Timestamp,
                )),
            isupport::MessageReferenceType::Timestamp => self.history.get_oldest_message(
                server,
                &history::Kind::Channel(channel.clone()),
                isupport::MessageReferenceType::Timestamp,
            ),
        };

        if let Some(oldest_message) = oldest_message {
            if matches!(
                message_reference_type,
                isupport::MessageReferenceType::MessageId
            ) {
                if let Some(id) = &oldest_message.id {
                    return MessageReference::MessageId(id.clone());
                }
            }

            MessageReference::Timestamp(
                oldest_message.server_time,
                oldest_message.id.clone().unwrap_or(":".to_string()),
            )
        } else {
            MessageReference::None
        }
    }

    pub fn broadcast_quit(
        &mut self,
        server: &Server,
        user: User,
        comment: Option<String>,
        user_channels: Vec<String>,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) {
        self.history.broadcast(
            server,
            Broadcast::Quit {
                user,
                comment,
                user_channels,
            },
            config,
            sent_time,
        );
    }

    pub fn broadcast_nickname(
        &mut self,
        server: &Server,
        old_nick: Nick,
        new_nick: Nick,
        ourself: bool,
        user_channels: Vec<String>,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) {
        self.history.broadcast(
            server,
            Broadcast::Nickname {
                new_nick,
                old_nick,
                ourself,
                user_channels,
            },
            config,
            sent_time,
        );
    }

    pub fn broadcast_invite(
        &mut self,
        server: &Server,
        inviter: Nick,
        channel: String,
        user_channels: Vec<String>,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) {
        self.history.broadcast(
            server,
            Broadcast::Invite {
                inviter,
                channel,
                user_channels,
            },
            config,
            sent_time,
        );
    }

    pub fn broadcast_change_host(
        &mut self,
        server: &Server,
        old_user: User,
        new_username: String,
        new_hostname: String,
        ourself: bool,
        user_channels: Vec<String>,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) {
        self.history.broadcast(
            server,
            Broadcast::ChangeHost {
                old_user,
                new_username,
                new_hostname,
                ourself,
                user_channels,
            },
            config,
            sent_time,
        );
    }

    pub fn broadcast_connecting(
        &mut self,
        server: &Server,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) {
        self.history
            .broadcast(server, Broadcast::Connecting, config, sent_time);
    }

    pub fn broadcast_connected(
        &mut self,
        server: &Server,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) {
        self.history
            .broadcast(server, Broadcast::Connected, config, sent_time);
    }

    pub fn broadcast_disconnected(
        &mut self,
        server: &Server,
        error: Option<String>,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) {
        self.history
            .broadcast(server, Broadcast::Disconnected { error }, config, sent_time);
    }

    pub fn broadcast_reconnected(
        &mut self,
        server: &Server,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) {
        self.history
            .broadcast(server, Broadcast::Reconnected, config, sent_time);
    }

    pub fn broadcast_connection_failed(
        &mut self,
        server: &Server,
        error: String,
        config: &Config,
        sent_time: DateTime<Utc>,
    ) {
        self.history.broadcast(
            server,
            Broadcast::ConnectionFailed { error },
            config,
            sent_time,
        );
    }

    fn get_focused_mut(
        &mut self,
        main_window: &Window,
    ) -> Option<(window::Id, pane_grid::Pane, &mut Pane)> {
        let (window, pane) = self.focus?;
        self.panes
            .get_mut(main_window.id, window, pane)
            .map(|state| (window, pane, state))
    }

    fn get_focused_with_history_mut(
        &mut self,
        main_window: &Window,
    ) -> Option<(pane_grid::Pane, &mut Pane, &mut history::Manager)> {
        let (window, pane) = self.focus?;
        self.panes
            .get_mut(main_window.id, window, pane)
            .map(|state| (pane, state, &mut self.history))
    }

    fn focus_pane(
        &mut self,
        main_window: &Window,
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Task<Message> {
        if self.focus != Some((window, pane)) {
            self.focus = Some((window, pane));

            if let Some(task) = self.panes.iter(main_window.id).find_map(|(w, p, state)| {
                (w == window && p == pane).then(|| {
                    state.buffer.focus().map(move |message| {
                        Message::Pane(window, pane::Message::Buffer(pane, message))
                    })
                })
            }) {
                return Task::batch(vec![task, window::gain_focus(window)]);
            }
        }

        Task::none()
    }

    fn maximize_pane(&mut self) {
        if self.is_pane_maximized() {
            self.panes.main.restore();
        } else if let Some((_, pane)) = self.focus {
            self.panes.main.maximize(pane);
        }
    }

    fn is_pane_maximized(&self) -> bool {
        self.panes.main.maximized().is_some()
    }

    fn new_pane(
        &mut self,
        axis: pane_grid::Axis,
        config: &Config,
        main_window: &Window,
    ) -> Task<Message> {
        if self
            .focus
            .filter(|(window, _)| *window == main_window.id)
            .is_some()
        {
            // If there is any focused pane on main window, split it
            return self.split_pane(axis, config, main_window);
        } else {
            // If there is no focused pane, split the last pane or create a new empty grid
            let pane = self.panes.main.iter().last().map(|(pane, _)| pane).cloned();

            if let Some(pane) = pane {
                let result = self
                    .panes
                    .main
                    .split(axis, pane, Pane::new(Buffer::Empty, config));
                self.last_changed = Some(Instant::now());

                if let Some((pane, _)) = result {
                    return self.focus_pane(main_window, main_window.id, pane);
                }
            } else {
                let (state, pane) = pane_grid::State::new(Pane::new(Buffer::Empty, config));
                self.panes.main = state;
                self.last_changed = Some(Instant::now());
                return self.focus_pane(main_window, main_window.id, pane);
            }
        }

        Task::none()
    }

    fn split_pane(
        &mut self,
        axis: pane_grid::Axis,
        config: &Config,
        main_window: &Window,
    ) -> Task<Message> {
        if let Some((window, pane)) = self.focus {
            if window == main_window.id {
                let result = self
                    .panes
                    .main
                    .split(axis, pane, Pane::new(Buffer::Empty, config));
                self.last_changed = Some(Instant::now());
                if let Some((pane, _)) = result {
                    return self.focus_pane(main_window, main_window.id, pane);
                }
            }
        }

        Task::none()
    }

    fn reset_pane(
        &mut self,
        main_window: &Window,
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Task<Message> {
        if let Some(state) = self.panes.get_mut(main_window.id, window, pane) {
            state.buffer.reset();
        }

        Task::none()
    }

    fn close_pane(
        &mut self,
        main_window: &Window,
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Task<Message> {
        self.last_changed = Some(Instant::now());

        if window == main_window.id {
            if let Some((_, sibling)) = self.panes.main.close(pane) {
                return self.focus_pane(main_window, main_window.id, sibling);
            } else if let Some(pane) = self.panes.main.get_mut(pane) {
                pane.buffer = Buffer::Empty;
            }
        } else if self.panes.popout.remove(&window).is_some() {
            return window::close(window);
        }

        Task::none()
    }

    fn popout_pane(&mut self, main_window: &Window) -> Task<Message> {
        if let Some((_, pane)) = self.focus.take() {
            if let Some((pane, _)) = self.panes.main.close(pane) {
                return self.open_popout_window(main_window, pane);
            }
        }

        Task::none()
    }

    fn merge_pane(&mut self, config: &Config, main_window: &Window) -> Task<Message> {
        if let Some((window, pane)) = self.focus.take() {
            if let Some(pane) = self
                .panes
                .popout
                .remove(&window)
                .and_then(|panes| panes.get(pane).cloned())
            {
                let task = match pane.buffer.data() {
                    Some(buffer) => self.open_buffer(buffer, pane.settings, main_window),
                    None if matches!(pane.buffer, Buffer::FileTransfers(_)) => {
                        self.toggle_file_transfers(config, main_window)
                    }
                    None => self.new_pane(pane_grid::Axis::Horizontal, config, main_window),
                };

                return Task::batch(vec![window::close(window), task]);
            }
        }

        Task::none()
    }

    pub fn track(&mut self) -> Task<Message> {
        let resources = self.panes.resources().collect();

        Task::batch(
            self.history
                .track(resources)
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
        buffers: &[data::Buffer],
        version: &Version,
        config: &Config,
        theme: &mut Theme,
        main_window: &Window,
    ) -> Task<Message> {
        if self.command_bar.is_some() {
            // Remove theme preview
            *theme = theme.selected();

            self.close_command_bar();
            // Refocus the pane so text input gets refocused
            self.focus
                .take()
                .map(|(window, pane)| self.focus_pane(main_window, window, pane))
                .unwrap_or(Task::none())
        } else {
            self.open_command_bar(buffers, version, config, main_window);
            Task::none()
        }
    }

    fn open_command_bar(
        &mut self,
        buffers: &[data::Buffer],
        version: &Version,
        config: &Config,
        main_window: &Window,
    ) {
        self.command_bar = Some(CommandBar::new(
            buffers,
            version,
            config,
            self.focus,
            self.buffer_resize_action(),
            main_window.id,
        ));
    }

    fn close_command_bar(&mut self) {
        self.command_bar = None;
    }

    fn buffer_resize_action(&self) -> data::buffer::Resize {
        let can_resize_buffer = self.focus.is_some() && self.panes.len() > 1;
        data::buffer::Resize::action(can_resize_buffer, self.is_pane_maximized())
    }

    pub fn receive_file_transfer(
        &mut self,
        server: &Server,
        request: file_transfer::ReceiveRequest,
        config: &Config,
    ) -> Option<Task<Message>> {
        if let Some(event) = self
            .file_transfers
            .receive(request.clone(), config.proxy.as_ref())
        {
            notification::file_transfer_request(&config.notifications, request.from, server);

            return Some(self.handle_file_transfer_event(server, event));
        }

        None
    }

    pub fn handle_file_transfer_event(
        &mut self,
        server: &Server,
        event: file_transfer::manager::Event,
    ) -> Task<Message> {
        match event {
            file_transfer::manager::Event::NewTransfer(transfer, task) => {
                match transfer.direction {
                    file_transfer::Direction::Received => {
                        self.record_message(
                            server,
                            data::Message::file_transfer_request_received(
                                &transfer.remote_user,
                                &transfer.filename,
                            ),
                        );
                    }
                    file_transfer::Direction::Sent => {
                        self.record_message(
                            server,
                            data::Message::file_transfer_request_sent(
                                &transfer.remote_user,
                                &transfer.filename,
                            ),
                        );
                    }
                }

                Task::run(task, Message::FileTransfer)
            }
        }
    }

    fn from_data(
        data: data::Dashboard,
        config: &Config,
        main_window: &Window,
    ) -> (Self, Task<Message>) {
        use pane_grid::Configuration;

        fn configuration(pane: data::Pane) -> Configuration<Pane> {
            match pane {
                data::Pane::Split { axis, ratio, a, b } => Configuration::Split {
                    axis: match axis {
                        data::pane::Axis::Horizontal => pane_grid::Axis::Horizontal,
                        data::pane::Axis::Vertical => pane_grid::Axis::Vertical,
                    },
                    ratio,
                    a: Box::new(configuration(*a)),
                    b: Box::new(configuration(*b)),
                },
                data::Pane::Buffer { buffer, settings } => {
                    Configuration::Pane(Pane::with_settings(Buffer::from(buffer), settings))
                }
                data::Pane::Empty => Configuration::Pane(Pane::with_settings(
                    Buffer::empty(),
                    buffer::Settings::default(),
                )),
                data::Pane::FileTransfers => Configuration::Pane(Pane::with_settings(
                    Buffer::FileTransfers(FileTransfers::new()),
                    buffer::Settings::default(),
                )),
            }
        }

        let mut dashboard = Self {
            panes: Panes {
                main: pane_grid::State::with_configuration(configuration(data.pane)),
                popout: HashMap::new(),
            },
            focus: None,
            side_menu: Sidebar::new(),
            history: history::Manager::default(),
            last_changed: None,
            command_bar: None,
            file_transfers: file_transfer::Manager::new(config.file_transfer.clone()),
            theme_editor: None,
        };

        let mut tasks = vec![];

        for pane in data.popout_panes {
            // Popouts are only a single pane
            let Configuration::Pane(pane) = configuration(pane) else {
                continue;
            };

            tasks.push(dashboard.open_popout_window(main_window, pane));
        }

        (dashboard, Task::batch(tasks))
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
                window::Event::Moved(_)
                | window::Event::Resized(_)
                | window::Event::Focused
                | window::Event::Unfocused
                | window::Event::Opened { .. } => {}
            }
        } else if self
            .theme_editor
            .as_ref()
            .map(|e| e.window == id)
            .unwrap_or_default()
        {
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

    pub fn exit(&mut self) -> Task<()> {
        let history = self.history.close_all();
        let last_changed = self.last_changed;
        let dashboard = data::Dashboard::from(&*self);

        let task = async move {
            history.await;

            if last_changed.is_some() {
                match dashboard.save().await {
                    Ok(_) => {
                        log::info!("dashboard saved");
                    }
                    Err(error) => {
                        log::warn!("error saving dashboard: {error}");
                    }
                }
            }
        };

        Task::perform(task, move |_| ())
    }

    fn open_popout_window(&mut self, main_window: &Window, pane: Pane) -> Task<Message> {
        self.last_changed = Some(Instant::now());

        let (window, task) = window::open(window::Settings {
            // Just big enough to show all components in combobox
            position: main_window
                .position
                .map(|point| window::Position::Specific(point + Vector::new(20.0, 20.0)))
                .unwrap_or_default(),
            exit_on_close_request: false,
            ..window::settings()
        });

        let (state, pane) = pane_grid::State::new(pane);
        self.panes.popout.insert(window, state);

        Task::batch(vec![
            task.then(|_| Task::none()),
            self.focus_pane(main_window, window, pane),
        ])
    }
}

impl<'a> From<&'a Dashboard> for data::Dashboard {
    fn from(dashboard: &'a Dashboard) -> Self {
        use pane_grid::Node;

        fn from_layout(panes: &pane_grid::State<Pane>, node: pane_grid::Node) -> data::Pane {
            match node {
                Node::Split {
                    axis, ratio, a, b, ..
                } => data::Pane::Split {
                    axis: match axis {
                        pane_grid::Axis::Horizontal => data::pane::Axis::Horizontal,
                        pane_grid::Axis::Vertical => data::pane::Axis::Vertical,
                    },
                    ratio,
                    a: Box::new(from_layout(panes, *a)),
                    b: Box::new(from_layout(panes, *b)),
                },
                Node::Pane(pane) => panes
                    .get(pane)
                    .cloned()
                    .map(data::Pane::from)
                    .unwrap_or(data::Pane::Empty),
            }
        }

        let layout = dashboard.panes.main.layout().clone();

        data::Dashboard {
            pane: from_layout(&dashboard.panes.main, layout),
            popout_panes: dashboard
                .panes
                .popout
                .values()
                .map(|state| from_layout(state, state.layout().clone()))
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct Panes {
    main: pane_grid::State<Pane>,
    popout: HashMap<window::Id, pane_grid::State<Pane>>,
}

impl Panes {
    fn len(&self) -> usize {
        self.main.panes.len() + self.popout.len()
    }

    fn get(
        &self,
        main_window: window::Id,
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Option<&Pane> {
        if main_window == window {
            self.main.get(pane)
        } else {
            self.popout.get(&window).and_then(|panes| panes.get(pane))
        }
    }

    fn get_mut(
        &mut self,
        main_window: window::Id,
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Option<&mut Pane> {
        if main_window == window {
            self.main.get_mut(pane)
        } else {
            self.popout
                .get_mut(&window)
                .and_then(|panes| panes.get_mut(pane))
        }
    }

    fn iter(
        &self,
        main_window: window::Id,
    ) -> impl Iterator<Item = (window::Id, pane_grid::Pane, &Pane)> {
        self.main
            .iter()
            .map(move |(pane, state)| (main_window, *pane, state))
            .chain(self.popout.iter().flat_map(|(window_id, panes)| {
                panes.iter().map(|(pane, state)| (*window_id, *pane, state))
            }))
    }

    fn resources(&self) -> impl Iterator<Item = data::history::Resource> + '_ {
        self.main.panes.values().filter_map(Pane::resource).chain(
            self.popout
                .values()
                .flat_map(|state| state.panes.values().filter_map(Pane::resource)),
        )
    }
}

fn all_buffers(clients: &client::Map, history: &history::Manager) -> Vec<data::Buffer> {
    clients
        .connected_servers()
        .flat_map(|server| {
            std::iter::once(data::Buffer::Server(server.clone()))
                .chain(
                    clients
                        .get_channels(server)
                        .iter()
                        .map(|channel| data::Buffer::Channel(server.clone(), channel.clone())),
                )
                .chain(
                    history
                        .get_unique_queries(server)
                        .into_iter()
                        .map(|nick| data::Buffer::Query(server.clone(), nick.clone())),
                )
        })
        .collect()
}

fn open_buffers(dashboard: &Dashboard, main_window: window::Id) -> Vec<data::Buffer> {
    dashboard
        .panes
        .iter(main_window)
        .filter_map(|(_, _, pane)| pane.buffer.data())
        .collect()
}

fn closed_buffers(
    dashboard: &Dashboard,
    main_window: window::Id,
    clients: &client::Map,
) -> Vec<data::Buffer> {
    let open_buffers = open_buffers(dashboard, main_window);

    all_buffers(clients, &dashboard.history)
        .into_iter()
        .filter(|buffer| !open_buffers.contains(buffer))
        .collect()
}

fn cycle_next_buffer(
    current: Option<&data::Buffer>,
    mut all: Vec<data::Buffer>,
    opened: &[data::Buffer],
) -> Option<data::Buffer> {
    all.retain(|buffer| Some(buffer) == current || !opened.contains(buffer));

    let next = || {
        let buffer = current?;
        let index = all.iter().position(|b| b == buffer)?;
        all.get(index + 1)
    };

    next().or_else(|| all.first()).cloned()
}

fn cycle_previous_buffer(
    current: Option<&data::Buffer>,
    mut all: Vec<data::Buffer>,
    opened: &[data::Buffer],
) -> Option<data::Buffer> {
    all.retain(|buffer| Some(buffer) == current || !opened.contains(buffer));

    let previous = || {
        let buffer = current?;
        let index = all.iter().position(|b| b == buffer).filter(|i| *i > 0)?;

        all.get(index - 1)
    };

    previous().or_else(|| all.last()).cloned()
}
