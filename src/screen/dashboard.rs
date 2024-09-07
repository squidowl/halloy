use chrono::{DateTime, Utc};
use data::environment::RELEASE_WEBSITE;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use data::file_transfer;
use data::history::manager::Broadcast;
use data::user::Nick;
use data::{client, environment, history, Config, Server, User, Version};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{column, container, row, Space};
use iced::{clipboard, Length, Task};

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
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    side_menu: Sidebar,
    history: history::Manager,
    last_changed: Option<Instant>,
    command_bar: Option<CommandBar>,
    file_transfers: file_transfer::Manager,
    theme_editor: Option<ThemeEditor>,
}

#[derive(Debug)]
pub enum Message {
    Pane(pane::Message),
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
        let (panes, _) = pane_grid::State::new(Pane::new(Buffer::Empty, config));

        let mut dashboard = Dashboard {
            panes,
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

    pub fn restore(dashboard: data::Dashboard, config: &Config) -> (Self, Task<Message>) {
        let mut dashboard = Dashboard::from_data(dashboard, config);

        let command = if let Some((pane, _)) = dashboard.panes.panes.iter().next() {
            Task::batch(vec![dashboard.focus_pane(*pane), dashboard.track()])
        } else {
            dashboard.track()
        };

        (dashboard, command)
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
            Message::Pane(message) => match message {
                pane::Message::PaneClicked(pane) => {
                    return (self.focus_pane(pane), None);
                }
                pane::Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                    self.panes.resize(split, ratio);
                    self.last_changed = Some(Instant::now());
                }
                pane::Message::PaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                    self.panes.drop(pane, target);
                    self.last_changed = Some(Instant::now());
                }
                pane::Message::PaneDragged(_) => {}
                pane::Message::ClosePane => {
                    if let Some(pane) = self.focus {
                        return (self.close_pane(pane), None);
                    }
                }
                pane::Message::SplitPane(axis) => {
                    return (self.split_pane(axis, config), None);
                }
                pane::Message::Buffer(id, message) => {
                    if let Some(pane) = self.panes.get_mut(id) {
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
                                        let command = data::Command::Whois(None, nick.to_string());

                                        let input = data::Input::command(buffer.clone(), command);

                                        if let Some(encoded) = input.encoded() {
                                            clients.send(input.buffer(), encoded);
                                        }

                                        if let Some(nick) = clients.nickname(buffer.server()) {
                                            let mut user = nick.to_owned().into();

                                            // Resolve our attributes if sending this message in a channel
                                            if let data::Buffer::Channel(server, channel) = &buffer
                                            {
                                                if let Some(user_with_attributes) = clients
                                                    .resolve_user_attributes(server, channel, &user)
                                                {
                                                    user = user_with_attributes.clone();
                                                }
                                            }

                                            if let Some(messages) = input.messages(user) {
                                                for message in messages {
                                                    self.history
                                                        .record_message(input.server(), message);
                                                }
                                            }
                                        }
                                    }
                                }
                                buffer::user_context::Event::OpenQuery(nick) => {
                                    if let Some(data) = pane.buffer.data() {
                                        let buffer =
                                            data::Buffer::Query(data.server().clone(), nick);
                                        return (self.open_buffer(buffer, config), None);
                                    }
                                }
                                buffer::user_context::Event::SingleClick(nick) => {
                                    let Some((_, pane, history)) =
                                        self.get_focused_with_history_mut()
                                    else {
                                        return (Task::none(), None);
                                    };

                                    return (
                                        pane.buffer.insert_user_to_input(nick, history).map(
                                            move |message| {
                                                Message::Pane(pane::Message::Buffer(id, message))
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
                                                        .map(|handle| handle.path().to_path_buf())
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

                        return (
                            command.map(move |message| {
                                Message::Pane(pane::Message::Buffer(id, message))
                            }),
                            None,
                        );
                    }
                }
                pane::Message::ToggleShowUserList => {
                    if let Some((_, pane)) = self.get_focused_mut() {
                        pane.update_settings(|settings| {
                            settings.channel.nicklist.toggle_visibility()
                        });
                        self.last_changed = Some(Instant::now());
                    }
                }
                pane::Message::ToggleShowTopic => {
                    if let Some((_, pane)) = self.get_focused_mut() {
                        pane.update_settings(|settings| settings.channel.topic.toggle_visibility());
                        self.last_changed = Some(Instant::now());
                    }
                }
                pane::Message::MaximizePane => self.maximize_pane(),
            },
            Message::Sidebar(message) => {
                let event = self.side_menu.update(message);

                match event {
                    sidebar::Event::Open(kind) => {
                        return (self.open_buffer(kind, config), None);
                    }
                    sidebar::Event::Replace(kind, pane) => {
                        if let Some(state) = self.panes.get_mut(pane) {
                            state.buffer = Buffer::from(kind);
                            self.last_changed = Some(Instant::now());
                            self.focus = None;
                            return (
                                Task::batch(vec![self.reset_pane(pane), self.focus_pane(pane)]),
                                None,
                            );
                        }
                    }
                    sidebar::Event::Close(pane) => {
                        if self.panes.close(pane).is_none() {
                            if let Some(state) = self.panes.get_mut(pane) {
                                state.buffer = Buffer::Empty;
                            }
                        }

                        self.last_changed = Some(Instant::now());

                        if self.focus == Some(pane) {
                            self.focus = None;
                        }
                    }
                    sidebar::Event::Swap(from, to) => {
                        self.panes.swap(from, to);
                        self.last_changed = Some(Instant::now());
                        return (self.focus_pane(from), None);
                    }
                    sidebar::Event::Leave(buffer) => {
                        return self.leave_buffer(clients, buffer);
                    }
                    sidebar::Event::ToggleFileTransfers => {
                        return (self.toggle_file_transfers(config), None);
                    }
                    sidebar::Event::ToggleCommandBar => {
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
                            return (window::close(editor.id), None);
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
                                command_bar::Buffer::New => {
                                    (self.new_pane(pane_grid::Axis::Horizontal, config), None)
                                }
                                command_bar::Buffer::Close => {
                                    if let Some(pane) = self.focus {
                                        (self.close_pane(pane), None)
                                    } else {
                                        (Task::none(), None)
                                    }
                                }
                                command_bar::Buffer::Replace(buffer) => {
                                    let mut commands = vec![];

                                    if let Some(pane) = self.focus.take() {
                                        if let Some(state) = self.panes.get_mut(pane) {
                                            state.buffer = Buffer::from(buffer);
                                            self.last_changed = Some(Instant::now());

                                            commands.extend(vec![
                                                self.reset_pane(pane),
                                                self.focus_pane(pane),
                                            ]);
                                        }
                                    }

                                    (Task::batch(commands), None)
                                }
                                command_bar::Buffer::ToggleFileTransfers => {
                                    (self.toggle_file_transfers(config), None)
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

                let mut move_focus = |direction: pane_grid::Direction| {
                    if let Some(pane) = self.focus.as_ref() {
                        if let Some(adjacent) = self.panes.adjacent(*pane, direction) {
                            return self.focus_pane(adjacent);
                        }
                    } else if let Some((pane, _)) = self.panes.panes.iter().next() {
                        return self.focus_pane(*pane);
                    }

                    Task::none()
                };

                match shortcut {
                    MoveUp => return (move_focus(pane_grid::Direction::Up), None),
                    MoveDown => return (move_focus(pane_grid::Direction::Down), None),
                    MoveLeft => return (move_focus(pane_grid::Direction::Left), None),
                    MoveRight => return (move_focus(pane_grid::Direction::Right), None),
                    CloseBuffer => {
                        if let Some(pane) = self.focus {
                            return (self.close_pane(pane), None);
                        }
                    }
                    MaximizeBuffer => {
                        if let Some(pane) = self.focus.as_ref() {
                            self.panes.maximize(*pane);
                        }
                    }
                    RestoreBuffer => {
                        self.panes.restore();
                    }
                    CycleNextBuffer => {
                        let all_buffers = all_buffers(clients, &self.history);
                        let open_buffers = open_buffers(self);

                        if let Some((pane, state)) = self.get_focused_mut() {
                            if let Some(buffer) = cycle_next_buffer(
                                state.buffer.data().as_ref(),
                                all_buffers,
                                &open_buffers,
                            ) {
                                state.buffer = Buffer::from(buffer);
                                self.focus = None;
                                return (self.focus_pane(pane), None);
                            }
                        }
                    }
                    CyclePreviousBuffer => {
                        let all_buffers = all_buffers(clients, &self.history);
                        let open_buffers = open_buffers(self);

                        if let Some((pane, state)) = self.get_focused_mut() {
                            if let Some(buffer) = cycle_previous_buffer(
                                state.buffer.data().as_ref(),
                                all_buffers,
                                &open_buffers,
                            ) {
                                state.buffer = Buffer::from(buffer);
                                self.focus = None;
                                return (self.focus_pane(pane), None);
                            }
                        }
                    }
                    LeaveBuffer => {
                        if let Some((_, state)) = self.get_focused_mut() {
                            if let Some(buffer) = state.buffer.data() {
                                return self.leave_buffer(clients, buffer);
                            }
                        }
                    }
                    ToggleNicklist => {
                        if let Some((_, pane)) = self.get_focused_mut() {
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
                                &closed_buffers(self, clients),
                                version,
                                config,
                                theme,
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
                        self.panes.restore();
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
                                tasks.push(window::close(editor.id));
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
        id: window::Id,
        theme: &'a Theme,
        config: &'a Config,
    ) -> Element<'a, Message> {
        if let Some(editor) = self.theme_editor.as_ref() {
            if editor.id == id {
                return editor.view(theme, config).map(Message::ThemeEditor);
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
    ) -> Element<'a, Message> {
        let focus = self.focus;

        let pane_grid: Element<_> = PaneGrid::new(&self.panes, |id, pane, maximized| {
            let is_focused = focus == Some(id);
            let panes = self.panes.len();
            pane.view(
                id,
                panes,
                is_focused,
                maximized,
                clients,
                &self.file_transfers,
                &self.history,
                &self.side_menu,
                config,
                theme,
            )
        })
        .on_click(pane::Message::PaneClicked)
        .on_resize(6, pane::Message::PaneResized)
        .on_drag(pane::Message::PaneDragged)
        .spacing(4)
        .into();

        let pane_grid = container(pane_grid.map(Message::Pane))
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
                        self.focus.is_some(),
                        self.buffer_resize_action(),
                        version,
                        config,
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
        clients: &data::client::Map,
        version: &Version,
        config: &Config,
        theme: &mut Theme,
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
                    self.toggle_command_bar(&closed_buffers(self, clients), version, config, theme)
                } else {
                    context_menu::close(Message::CloseContextMenu)
                }
            }
            Copy => selectable_text::selected(Message::SelectedText),
            Home => self
                .get_focused_mut()
                .map(|(id, pane)| {
                    pane.buffer
                        .scroll_to_start()
                        .map(move |message| Message::Pane(pane::Message::Buffer(id, message)))
                })
                .unwrap_or_else(Task::none),
            End => self
                .get_focused_mut()
                .map(|(pane, state)| {
                    state
                        .buffer
                        .scroll_to_end()
                        .map(move |message| Message::Pane(pane::Message::Buffer(pane, message)))
                })
                .unwrap_or_else(Task::none),
        }
    }

    // TODO: Perhaps rewrite this, i just did this quickly.
    fn toggle_file_transfers(&mut self, config: &Config) -> Task<Message> {
        let panes = self.panes.clone();

        // If file transfers already is open, we close it.
        for (id, pane) in panes.iter() {
            if let Buffer::FileTransfers(_) = pane.buffer {
                return self.close_pane(*id);
            }
        }

        // If we only have one pane, and its empty, we replace it.
        if self.panes.len() == 1 {
            for (id, pane) in panes.iter() {
                if let Buffer::Empty = &pane.buffer {
                    self.panes.panes.entry(*id).and_modify(|p| {
                        *p = Pane::new(Buffer::FileTransfers(FileTransfers::new()), config)
                    });
                    self.last_changed = Some(Instant::now());

                    return self.focus_pane(*id);
                }
            }
        }

        let mut commands = vec![];
        let _ = self.new_pane(pane_grid::Axis::Vertical, config);

        if let Some(pane) = self.focus.take() {
            if let Some(state) = self.panes.get_mut(pane) {
                state.buffer = Buffer::FileTransfers(FileTransfers::new());
                self.last_changed = Some(Instant::now());

                commands.extend(vec![self.reset_pane(pane), self.focus_pane(pane)]);
            }
        }

        Task::batch(commands)
    }

    fn open_buffer(&mut self, kind: data::Buffer, config: &Config) -> Task<Message> {
        let panes = self.panes.clone();

        // If channel already is open, we focus it.
        for (id, pane) in panes.iter() {
            if pane.buffer.data().as_ref() == Some(&kind) {
                self.focus = Some(*id);

                return self.focus_pane(*id);
            }
        }

        // If we only have one pane, and its empty, we replace it.
        if self.panes.len() == 1 {
            for (id, pane) in panes.iter() {
                if let Buffer::Empty = &pane.buffer {
                    self.panes
                        .panes
                        .entry(*id)
                        .and_modify(|p| *p = Pane::new(Buffer::from(kind), config));
                    self.last_changed = Some(Instant::now());

                    return self.focus_pane(*id);
                }
            }
        }

        // Default split could be a config option.
        let axis = pane_grid::Axis::Horizontal;
        let pane_to_split = {
            if let Some(pane) = self.focus {
                pane
            } else if let Some(pane) = self.panes.panes.keys().last() {
                *pane
            } else {
                log::error!("Didn't find any panes");
                return Task::none();
            }
        };

        let result = self
            .panes
            .split(axis, pane_to_split, Pane::new(Buffer::from(kind), config));
        self.last_changed = Some(Instant::now());

        if let Some((pane, _)) = result {
            return self.focus_pane(pane);
        }

        Task::none()
    }

    pub fn leave_buffer(
        &mut self,
        clients: &mut data::client::Map,
        buffer: data::Buffer,
    ) -> (Task<Message>, Option<Event>) {
        let pane = self.panes.iter().find_map(|(pane, state)| {
            (state.buffer.data().as_ref() == Some(&buffer)).then_some(*pane)
        });

        // Close pane
        if let Some(pane) = pane {
            if self.panes.close(pane).is_none() {
                if let Some(state) = self.panes.get_mut(pane) {
                    state.buffer = Buffer::Empty;
                }
            }
            self.last_changed = Some(Instant::now());

            if self.focus == Some(pane) {
                self.focus = None;
            }
        }

        match buffer.clone() {
            data::Buffer::Server(server) => (Task::none(), Some(Event::QuitServer(server))),
            data::Buffer::Channel(server, channel) => {
                // Send part & close history file
                let command = data::Command::Part(channel.clone(), None);
                let input = data::Input::command(buffer.clone(), command);

                if let Some(encoded) = input.encoded() {
                    clients.send(&buffer, encoded);
                }

                (
                    self.history
                        .close(server, history::Kind::Channel(channel))
                        .map(|task| Task::perform(task, |_| Message::CloseHistory))
                        .unwrap_or_else(Task::none),
                    None,
                )
            }
            data::Buffer::Query(server, nick) => {
                // No PART to send, just close history
                (
                    self.history
                        .close(server, history::Kind::Query(nick))
                        .map(|task| Task::perform(task, |_| Message::CloseHistory))
                        .unwrap_or_else(Task::none),
                    None,
                )
            }
        }
    }

    pub fn record_message(&mut self, server: &Server, message: data::Message) {
        self.history.record_message(server, message);
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

    fn get_focused_mut(&mut self) -> Option<(pane_grid::Pane, &mut Pane)> {
        let pane = self.focus?;
        self.panes.get_mut(pane).map(|state| (pane, state))
    }

    fn get_focused_with_history_mut(
        &mut self,
    ) -> Option<(pane_grid::Pane, &mut Pane, &mut history::Manager)> {
        let pane = self.focus?;
        self.panes
            .get_mut(pane)
            .map(|state| (pane, state, &mut self.history))
    }

    fn focus_pane(&mut self, pane: pane_grid::Pane) -> Task<Message> {
        if self.focus != Some(pane) {
            self.focus = Some(pane);

            self.panes
                .iter()
                .find_map(|(p, state)| {
                    (*p == pane).then(|| {
                        state
                            .buffer
                            .focus()
                            .map(move |message| Message::Pane(pane::Message::Buffer(pane, message)))
                    })
                })
                .unwrap_or(Task::none())
        } else {
            Task::none()
        }
    }

    fn maximize_pane(&mut self) {
        if self.is_pane_maximized() {
            self.panes.restore();
        } else if let Some(pane) = self.focus {
            self.panes.maximize(pane);
        }
    }

    fn is_pane_maximized(&self) -> bool {
        self.panes.maximized().is_some()
    }

    fn new_pane(&mut self, axis: pane_grid::Axis, config: &Config) -> Task<Message> {
        if self.focus.is_some() {
            // If there is any focused pane, split it
            return self.split_pane(axis, config);
        } else {
            // If there is no focused pane, split the last pane or create a new empty grid
            let pane = self.panes.iter().last().map(|(pane, _)| pane).cloned();

            if let Some(pane) = pane {
                let result = self
                    .panes
                    .split(axis, pane, Pane::new(Buffer::Empty, config));
                self.last_changed = Some(Instant::now());

                if let Some((pane, _)) = result {
                    return self.focus_pane(pane);
                }
            } else {
                let (state, pane) = pane_grid::State::new(Pane::new(Buffer::Empty, config));
                self.panes = state;
                self.last_changed = Some(Instant::now());
                return self.focus_pane(pane);
            }
        }

        Task::none()
    }

    fn split_pane(&mut self, axis: pane_grid::Axis, config: &Config) -> Task<Message> {
        if let Some(pane) = self.focus {
            let result = self
                .panes
                .split(axis, pane, Pane::new(Buffer::Empty, config));
            self.last_changed = Some(Instant::now());
            if let Some((pane, _)) = result {
                return self.focus_pane(pane);
            }
        }

        Task::none()
    }

    fn reset_pane(&mut self, pane: pane_grid::Pane) -> Task<Message> {
        if let Some((_, state)) = self.panes.iter_mut().find(|(p, _)| **p == pane) {
            state.buffer.reset();
        }

        Task::none()
    }

    fn close_pane(&mut self, pane: pane_grid::Pane) -> Task<Message> {
        self.last_changed = Some(Instant::now());

        if let Some((_, sibling)) = self.panes.close(pane) {
            return self.focus_pane(sibling);
        } else if let Some(pane) = self.panes.get_mut(pane) {
            pane.buffer = Buffer::Empty;
        }

        Task::none()
    }

    pub fn track(&mut self) -> Task<Message> {
        let resources = self
            .panes
            .iter()
            .filter_map(|(_, pane)| pane.resource())
            .collect();

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
    ) -> Task<Message> {
        if self.command_bar.is_some() {
            // Remove theme preview
            *theme = theme.selected();

            self.close_command_bar();
            // Refocus the pane so text input gets refocused
            self.focus
                .take()
                .map(|pane| self.focus_pane(pane))
                .unwrap_or(Task::none())
        } else {
            self.open_command_bar(buffers, version, config);
            Task::none()
        }
    }

    fn open_command_bar(&mut self, buffers: &[data::Buffer], version: &Version, config: &Config) {
        self.command_bar = Some(CommandBar::new(
            buffers,
            version,
            config,
            self.focus.is_some(),
            self.buffer_resize_action(),
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

    fn from_data(dashboard: data::Dashboard, config: &Config) -> Self {
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

        Self {
            panes: pane_grid::State::with_configuration(configuration(dashboard.pane)),
            focus: None,
            side_menu: Sidebar::new(),
            history: history::Manager::default(),
            last_changed: None,
            command_bar: None,
            file_transfers: file_transfer::Manager::new(config.file_transfer.clone()),
            theme_editor: None,
        }
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
        if Some(id) == self.theme_editor.as_ref().map(|e| e.id) {
            match event {
                window::Event::CloseRequested => {
                    if let Some(editor) = self.theme_editor.take() {
                        *theme = theme.selected();
                        return window::close(editor.id);
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
            window::gain_focus(editor.id)
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

        let layout = dashboard.panes.layout().clone();

        data::Dashboard {
            pane: from_layout(&dashboard.panes, layout),
        }
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

fn open_buffers(dashboard: &Dashboard) -> Vec<data::Buffer> {
    dashboard
        .panes
        .iter()
        .filter_map(|(_, pane)| pane.buffer.data())
        .collect()
}

fn closed_buffers(dashboard: &Dashboard, clients: &client::Map) -> Vec<data::Buffer> {
    let open_buffers = open_buffers(dashboard);

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
