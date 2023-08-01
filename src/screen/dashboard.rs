mod command_bar;
pub mod pane;
pub mod side_menu;

use std::time::{Duration, Instant};

use data::history::manager::Broadcast;
use data::user::Nick;
use data::{client, history, server, Config, Server, User};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{container, row, Space};
use iced::{clipboard, window, Command, Length};

use self::command_bar::CommandBar;
use self::pane::Pane;
use self::side_menu::SideMenu;
use crate::buffer::{self, Buffer};
use crate::widget::{anchored_overlay, selectable_text, shortcut, Collection, Element};
use crate::{event, theme, Theme};

const SAVE_AFTER: Duration = Duration::from_secs(3);

pub struct Dashboard {
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    side_menu: SideMenu,
    history: history::Manager,
    last_changed: Option<Instant>,
    command_bar: Option<CommandBar>,
}

#[derive(Debug)]
pub enum Message {
    Pane(pane::Message),
    SideMenu(side_menu::Message),
    SelectedText(Vec<(f32, String)>),
    History(history::manager::Message),
    Close,
    DashboardSaved(Result<(), data::dashboard::Error>),
    CloseHistory,
    QuitServer,
    Command(command_bar::Message),
    Shortcut(shortcut::Command),
}

impl Dashboard {
    pub fn empty(config: &Config) -> (Self, Command<Message>) {
        let (panes, _) = pane_grid::State::new(Pane::new(Buffer::Empty, config));

        let mut dashboard = Dashboard {
            panes,
            focus: None,
            side_menu: SideMenu::new(),
            history: history::Manager::default(),
            last_changed: None,
            command_bar: None,
        };

        let command = dashboard.track();

        (dashboard, command)
    }

    pub fn restore(dashboard: data::Dashboard) -> (Self, Command<Message>) {
        let mut dashboard = Dashboard::from(dashboard);

        let command = dashboard.track();

        (dashboard, command)
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut client::Map,
        servers: &mut server::Map,
        theme: &mut Theme,
        config: &Config,
    ) -> Command<Message> {
        match message {
            Message::Pane(message) => match message {
                pane::Message::PaneClicked(pane) => {
                    return self.focus_pane(pane);
                }
                pane::Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                    self.panes.resize(&split, ratio);
                    self.last_changed = Some(Instant::now());
                }
                pane::Message::PaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                    self.panes.drop(&pane, target);
                    self.last_changed = Some(Instant::now());
                }
                pane::Message::PaneDragged(_) => {}
                pane::Message::ClosePane => {
                    if let Some(pane) = self.focus {
                        return self.close_pane(pane);
                    }
                }
                pane::Message::SplitPane(axis) => {
                    return self.split_pane(axis, config);
                }
                pane::Message::Buffer(id, message) => {
                    if let Some(pane) = self.panes.get_mut(&id) {
                        let (command, event) =
                            pane.buffer.update(message, clients, &mut self.history);

                        if let Some(buffer::Event::UserContext(event)) = event {
                            match event {
                                buffer::user_context::Event::SendWhois(user) => {
                                    if let Some(buffer) = pane.buffer.data() {
                                        let command =
                                            data::Command::Whois(None, user.nickname().to_string());

                                        let input = data::Input::command(buffer, command);

                                        if let Some(encoded) = input.encoded() {
                                            clients.send(input.buffer(), encoded);
                                        }

                                        if let Some(message) = clients
                                            .nickname(input.server())
                                            .and_then(|nick| input.message(nick))
                                        {
                                            self.history.record_message(input.server(), message);
                                        }
                                    }
                                }
                                buffer::user_context::Event::OpenQuery(user) => {
                                    if let Some(data) = pane.buffer.data() {
                                        let buffer = data::Buffer::Query(
                                            data.server().clone(),
                                            user.nickname().to_owned(),
                                        );
                                        return self.open_buffer(buffer, config);
                                    }
                                }
                                buffer::user_context::Event::SingleClick(user) => {
                                    let Some((_, pane)) = self.get_focused_mut() else {
                                        return Command::none();
                                    };

                                    return pane.buffer.insert_user_to_input(user).map(
                                        move |message| {
                                            Message::Pane(pane::Message::Buffer(id, message))
                                        },
                                    );
                                }
                            }
                        }

                        return command
                            .map(move |message| Message::Pane(pane::Message::Buffer(id, message)));
                    }
                }
                pane::Message::ToggleShowUserList => {
                    if let Some((_, pane)) = self.get_focused_mut() {
                        pane.update_settings(|settings| settings.channel.users.toggle_visibility());
                        self.last_changed = Some(Instant::now());
                    }
                }
                pane::Message::MaximizePane => self.maximize_pane(),
            },
            Message::SideMenu(message) => {
                let event = self.side_menu.update(message);

                match event {
                    side_menu::Event::Open(kind) => {
                        return self.open_buffer(kind, config);
                    }
                    side_menu::Event::Replace(kind, pane) => {
                        if let Some(state) = self.panes.get_mut(&pane) {
                            state.buffer = Buffer::from(kind);
                            self.last_changed = Some(Instant::now());
                            self.focus = None;
                            return Command::batch(vec![
                                self.reset_pane(pane),
                                self.focus_pane(pane),
                            ]);
                        }
                    }
                    side_menu::Event::Close(pane) => {
                        self.panes.close(&pane);
                        self.last_changed = Some(Instant::now());

                        if self.focus == Some(pane) {
                            self.focus = None;
                        }
                    }
                    side_menu::Event::Swap(from, to) => {
                        self.panes.swap(&from, &to);
                        self.last_changed = Some(Instant::now());
                        return self.focus_pane(from);
                    }
                    side_menu::Event::Leave(buffer) => {
                        let pane = self.panes.iter().find_map(|(pane, state)| {
                            (state.buffer.data().as_ref() == Some(&buffer)).then_some(*pane)
                        });

                        // Close pane
                        if let Some(pane) = pane {
                            if self.panes.close(&pane).is_none() {
                                if let Some(state) = self.panes.get_mut(&pane) {
                                    state.buffer = Buffer::Empty;
                                }
                            }
                            self.last_changed = Some(Instant::now());

                            if self.focus == Some(pane) {
                                self.focus = None;
                            }
                        }

                        match buffer.clone() {
                            data::Buffer::Server(server) => {
                                // Remove server connection

                                // Removing from servers kills stream subscription
                                servers.remove(&server);

                                // Remove from clients pool to fully drop it
                                let _server = server.clone();
                                let quit = clients
                                    .remove(&server)
                                    .map(move |connection| async move {
                                        connection.quit().await;

                                        log::info!("[{_server}] quit");
                                    })
                                    .map(|task| Command::perform(task, |_| Message::QuitServer))
                                    .unwrap_or_else(Command::none);

                                // Close history for server
                                let close_history = self
                                    .history
                                    .close_server(server)
                                    .map(|task| Command::perform(task, |_| Message::CloseHistory))
                                    .unwrap_or_else(Command::none);

                                return Command::batch(vec![quit, close_history]);
                            }
                            data::Buffer::Channel(server, channel) => {
                                // Send part & close history file
                                let command = data::Command::Part(channel.clone(), None);
                                let input = data::Input::command(buffer.clone(), command);

                                if let Some(encoded) = input.encoded() {
                                    clients.send(&buffer, encoded);
                                }

                                return self
                                    .history
                                    .close(server, history::Kind::Channel(channel))
                                    .map(|task| Command::perform(task, |_| Message::CloseHistory))
                                    .unwrap_or_else(Command::none);
                            }
                            data::Buffer::Query(server, nick) => {
                                // No PART to send, just close history
                                return self
                                    .history
                                    .close(server, history::Kind::Query(nick))
                                    .map(|task| Command::perform(task, |_| Message::CloseHistory))
                                    .unwrap_or_else(Command::none);
                            }
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
                    return clipboard::write(contents);
                }
            }
            Message::History(message) => {
                self.history.update(message);
            }
            Message::Close => {
                return window::close();
            }
            Message::DashboardSaved(Ok(_)) => {
                log::info!("dashboard saved");
            }
            Message::DashboardSaved(Err(error)) => {
                log::warn!("error saving dashboard: {error}");
            }
            Message::CloseHistory => {}
            Message::QuitServer => {}
            Message::Command(message) => {
                let Some(command_bar) = &mut self.command_bar else {
                    return Command::none();
                };

                match command_bar.update(message) {
                    Some(command_bar::Event::ThemePreview(preview)) => match preview {
                        Some(preview) => *theme = theme.preview(preview),
                        None => *theme = theme.selected(),
                    },
                    Some(command_bar::Event::Command(command)) => {
                        let command = match command {
                            command_bar::Command::Buffer(command) => match command {
                                command_bar::Buffer::Maximize(_) => {
                                    self.maximize_pane();
                                    Command::none()
                                }
                                command_bar::Buffer::New => {
                                    self.new_pane(pane_grid::Axis::Horizontal, config)
                                }
                                command_bar::Buffer::Close => {
                                    if let Some(pane) = self.focus {
                                        self.close_pane(pane)
                                    } else {
                                        Command::none()
                                    }
                                }
                                command_bar::Buffer::Replace(buffer) => {
                                    let mut commands = vec![];

                                    if let Some(pane) = self.focus.take() {
                                        if let Some(state) = self.panes.get_mut(&pane) {
                                            state.buffer = Buffer::from(buffer);
                                            self.last_changed = Some(Instant::now());

                                            commands.extend(vec![
                                                self.reset_pane(pane),
                                                self.focus_pane(pane),
                                            ]);
                                        }
                                    }

                                    Command::batch(commands)
                                }
                            },
                            command_bar::Command::Configuration(command) => match command {
                                command_bar::Configuration::Open => {
                                    let _ = open::that(Config::config_dir());
                                    Command::none()
                                }
                            },
                            command_bar::Command::UI(command) => match command {
                                command_bar::Ui::ToggleSidebarVisibility => {
                                    self.side_menu.toggle_visibility();
                                    Command::none()
                                }
                            },
                            command_bar::Command::Theme(command) => match command {
                                command_bar::Theme::Switch(new) => {
                                    *theme = Theme::from(new);
                                    Command::none()
                                }
                            },
                        };

                        return Command::batch(vec![
                            command,
                            self.toggle_command_bar(&closed_buffers(self, clients), config, theme),
                        ]);
                    }
                    Some(command_bar::Event::Unfocused) => {
                        return self.toggle_command_bar(
                            &closed_buffers(self, clients),
                            config,
                            theme,
                        );
                    }
                    None => {}
                }
            }
            Message::Shortcut(shortcut) => {
                use shortcut::Command::*;

                let mut move_focus = |direction: pane_grid::Direction| {
                    if let Some(pane) = self.focus.as_ref() {
                        if let Some(adjacent) = self.panes.adjacent(pane, direction) {
                            return self.focus_pane(adjacent);
                        }
                    } else if let Some((pane, _)) = self.panes.panes.iter().next() {
                        return self.focus_pane(*pane);
                    }

                    Command::none()
                };

                match shortcut {
                    MoveUp => return move_focus(pane_grid::Direction::Up),
                    MoveDown => return move_focus(pane_grid::Direction::Down),
                    MoveLeft => return move_focus(pane_grid::Direction::Left),
                    MoveRight => return move_focus(pane_grid::Direction::Right),
                    CloseBuffer => {
                        if let Some(pane) = self.focus {
                            return self.close_pane(pane);
                        }
                    }
                    MaximizeBuffer => {
                        if let Some(pane) = self.focus.as_ref() {
                            self.panes.maximize(pane);
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
                                return self.focus_pane(pane);
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
                                return self.focus_pane(pane);
                            }
                        }
                    }
                    ToggleNickList => {
                        if let Some((_, pane)) = self.get_focused_mut() {
                            pane.update_settings(|settings| {
                                settings.channel.users.visible = !settings.channel.users.visible
                            });
                        }
                    }
                }
            }
        }

        Command::none()
    }

    pub fn view<'a>(
        &'a self,
        clients: &'a client::Map,
        config: &'a Config,
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
                &self.history,
                config,
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
                clients,
                &self.history,
                &self.panes,
                self.focus,
                config.dashboard.sidebar_default_action,
            )
            .map(|e| e.map(Message::SideMenu));

        // The height margin varies across different operating systems due to design differences.
        // For instance, on macOS, the menubar is hidden, resulting in a need for additional padding to accommodate the
        // space occupied by the traffic light buttons.
        let height_margin = if cfg!(target_os = "macos") { 20 } else { 0 };

        let base = row![]
            .push_maybe(side_menu)
            .push(pane_grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([height_margin, 0, 0, 0]);

        let base = if let Some(command_bar) = self.command_bar.as_ref() {
            let background = anchored_overlay(
                base,
                container(Space::new(Length::Fill, Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::Container::SemiTransparent),
                anchored_overlay::Anchor::BelowTopCentered,
                0.0,
            );

            // Command bar
            anchored_overlay(
                background,
                command_bar
                    .view(
                        &all_buffers(clients, &self.history),
                        self.focus.is_some(),
                        self.buffer_resize_action(),
                        config,
                    )
                    .map(Message::Command),
                anchored_overlay::Anchor::BelowTopCentered,
                10.0,
            )
        } else {
            base.into()
        };

        shortcut(base, config.keys.shortcuts(), Message::Shortcut)
    }

    pub fn handle_event(
        &mut self,
        event: event::Event,
        clients: &data::client::Map,
        config: &Config,
        theme: &mut Theme,
    ) -> Command<Message> {
        use event::Event::*;

        match event {
            Escape => {
                // Order of operations
                //
                // - Close command bar
                // - Restore maximized pane
                // - Unfocus
                if self.command_bar.is_some() {
                    return self.toggle_command_bar(&closed_buffers(self, clients), config, theme);
                } else if self.is_pane_maximized() {
                    self.panes.restore();
                } else {
                    self.focus = None;
                }

                Command::none()
            }
            Copy => selectable_text::selected(Message::SelectedText),
            Home => self
                .get_focused_mut()
                .map(|(id, pane)| {
                    pane.buffer
                        .scroll_to_start()
                        .map(move |message| Message::Pane(pane::Message::Buffer(id, message)))
                })
                .unwrap_or_else(Command::none),
            End => self
                .get_focused_mut()
                .map(|(pane, state)| {
                    state
                        .buffer
                        .scroll_to_end()
                        .map(move |message| Message::Pane(pane::Message::Buffer(pane, message)))
                })
                .unwrap_or_else(Command::none),
            CloseRequested => {
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

                Command::perform(task, |_| Message::Close)
            }
            CommandBar => self.toggle_command_bar(&closed_buffers(self, clients), config, theme),
        }
    }

    fn open_buffer(&mut self, kind: data::Buffer, config: &Config) -> Command<Message> {
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
                return Command::none();
            }
        };

        let result = self
            .panes
            .split(axis, &pane_to_split, Pane::new(Buffer::from(kind), config));
        self.last_changed = Some(Instant::now());

        if let Some((pane, _)) = result {
            return self.focus_pane(pane);
        }

        Command::none()
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
    ) {
        self.history.broadcast(
            server,
            Broadcast::Quit {
                user,
                comment,
                user_channels,
            },
        );
    }

    pub fn broadcast_nickname(
        &mut self,
        server: &Server,
        old_nick: Nick,
        new_nick: Nick,
        ourself: bool,
        user_channels: Vec<String>,
    ) {
        self.history.broadcast(
            server,
            Broadcast::Nickname {
                new_nick,
                old_nick,
                ourself,
                user_channels,
            },
        );
    }

    pub fn broadcast_connecting(&mut self, server: &Server) {
        self.history.broadcast(server, Broadcast::Connecting);
    }

    pub fn broadcast_connected(&mut self, server: &Server) {
        self.history.broadcast(server, Broadcast::Connected);
    }

    pub fn broadcast_disconnected(&mut self, server: &Server, error: Option<String>) {
        self.history
            .broadcast(server, Broadcast::Disconnected { error });
    }

    pub fn broadcast_reconnected(&mut self, server: &Server) {
        self.history.broadcast(server, Broadcast::Reconnected);
    }

    pub fn broadcast_connection_failed(&mut self, server: &Server, error: String) {
        self.history
            .broadcast(server, Broadcast::ConnectionFailed { error });
    }

    fn get_focused_mut(&mut self) -> Option<(pane_grid::Pane, &mut Pane)> {
        let pane = self.focus?;
        self.panes.get_mut(&pane).map(|state| (pane, state))
    }

    fn focus_pane(&mut self, pane: pane_grid::Pane) -> Command<Message> {
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
                .unwrap_or(Command::none())
        } else {
            Command::none()
        }
    }

    fn maximize_pane(&mut self) {
        if self.is_pane_maximized() {
            self.panes.restore();
        } else if let Some(pane) = self.focus {
            self.panes.maximize(&pane);
        }
    }

    fn is_pane_maximized(&self) -> bool {
        self.panes.maximized().is_some()
    }

    fn new_pane(&mut self, axis: pane_grid::Axis, config: &Config) -> Command<Message> {
        if self.focus.is_some() {
            // If there is any focused pane, split it
            return self.split_pane(axis, config);
        } else {
            // If there is no focused pane, split the last pane or create a new empty grid
            let pane = self.panes.iter().last().map(|(pane, _)| pane).cloned();

            if let Some(pane) = pane {
                let result = self
                    .panes
                    .split(axis, &pane, Pane::new(Buffer::Empty, config));
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

        Command::none()
    }

    fn split_pane(&mut self, axis: pane_grid::Axis, config: &Config) -> Command<Message> {
        if let Some(pane) = self.focus {
            let result = self
                .panes
                .split(axis, &pane, Pane::new(Buffer::Empty, config));
            self.last_changed = Some(Instant::now());
            if let Some((pane, _)) = result {
                return self.focus_pane(pane);
            }
        }

        Command::none()
    }

    fn reset_pane(&mut self, pane: pane_grid::Pane) -> Command<Message> {
        self.panes
            .iter()
            .find_map(|(p, state)| {
                (*p == pane).then(|| {
                    state
                        .buffer
                        .reset()
                        .map(move |message| Message::Pane(pane::Message::Buffer(pane, message)))
                })
            })
            .unwrap_or(Command::none())
    }

    fn close_pane(&mut self, pane: pane_grid::Pane) -> Command<Message> {
        self.last_changed = Some(Instant::now());

        if let Some((_, sibling)) = self.panes.close(&pane) {
            return self.focus_pane(sibling);
        } else if let Some(pane) = self.panes.get_mut(&pane) {
            pane.buffer = Buffer::Empty;
        }

        Command::none()
    }

    pub fn track(&mut self) -> Command<Message> {
        let resources = self
            .panes
            .iter()
            .filter_map(|(_, pane)| pane.resource())
            .collect();

        Command::batch(
            self.history
                .track(resources)
                .into_iter()
                .map(|fut| Command::perform(fut, Message::History))
                .collect::<Vec<_>>(),
        )
    }

    pub fn tick(&mut self, now: Instant) -> Command<Message> {
        let history = Command::batch(
            self.history
                .tick(now.into())
                .into_iter()
                .map(|task| Command::perform(task, Message::History))
                .collect::<Vec<_>>(),
        );

        if let Some(last_changed) = self.last_changed {
            if now.duration_since(last_changed) >= SAVE_AFTER {
                let dashboard = data::Dashboard::from(&*self);

                self.last_changed = None;

                return Command::batch(vec![
                    Command::perform(dashboard.save(), Message::DashboardSaved),
                    history,
                ]);
            }
        }

        history
    }

    pub fn toggle_command_bar(
        &mut self,
        buffers: &[data::Buffer],
        config: &Config,
        theme: &mut Theme,
    ) -> Command<Message> {
        if self.command_bar.is_some() {
            // Remove theme preview
            *theme = theme.selected();

            self.close_command_bar();
            // Refocus the pane so text input gets refocused
            self.focus
                .take()
                .map(|pane| self.focus_pane(pane))
                .unwrap_or(Command::none())
        } else {
            self.open_command_bar(buffers, config);

            Command::none()
        }
    }

    fn open_command_bar(&mut self, buffers: &[data::Buffer], config: &Config) {
        self.command_bar = Some(CommandBar::new(
            buffers,
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
}

impl From<data::Dashboard> for Dashboard {
    fn from(dashboard: data::Dashboard) -> Self {
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
            }
        }

        Self {
            panes: pane_grid::State::with_configuration(configuration(dashboard.pane)),
            focus: None,
            side_menu: SideMenu::new(),
            history: history::Manager::default(),
            last_changed: None,
            command_bar: None,
        }
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
                    .get(&pane)
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
