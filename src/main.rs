#![allow(clippy::large_enum_variant, clippy::too_many_arguments)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod buffer;
mod event;
mod font;
mod icon;
mod logger;
mod modal;
mod notification;
mod screen;
mod stream;
mod theme;
mod url;
mod widget;
mod window;

use std::env;
use std::time::{Duration, Instant};

use chrono::Utc;
use data::config::{self, Config};
use data::isupport::{ChatHistorySubcommand, MessageReference};
use data::version::Version;
use data::{environment, history, server, version, Url, User};
use iced::widget::{column, container};
use iced::{padding, Length, Subscription, Task};
use screen::{dashboard, help, migration, welcome};

use self::event::{events, Event};
use self::modal::Modal;
use self::theme::Theme;
use self::widget::Element;
use self::window::Window;

pub fn main() -> iced::Result {
    let mut args = env::args();
    args.next();

    let version = args
        .next()
        .map(|s| s == "--version" || s == "-V")
        .unwrap_or_default();

    if version {
        println!("halloy {}", environment::formatted_version());

        return Ok(());
    }

    #[cfg(debug_assertions)]
    let is_debug = true;
    #[cfg(not(debug_assertions))]
    let is_debug = false;

    // Prepare notifications.
    notification::prepare();

    logger::setup(is_debug).expect("setup logging");
    log::info!("halloy {} has started", environment::formatted_version());
    log::info!("config dir: {:?}", environment::config_dir());
    log::info!("data dir: {:?}", environment::data_dir());

    let config_load = Config::load();

    // DANGER ZONE - font must be set using config
    // before we do any iced related stuff w/ it
    font::set(config_load.as_ref().ok());

    let destination = data::Url::find_in(std::env::args());
    if let Some(loc) = &destination {
        if ipc::connect_and_send(loc.to_string()) {
            return Ok(());
        }
    }

    // TODO: Renable persistant window position and size:
    // Winit currently has a bug with resize and move events.
    // Until it have been fixed, the persistant position and size has been disabled.
    //
    // let window_load = Window::load().unwrap_or_default();

    if let Err(error) = iced::daemon("Halloy", Halloy::update, Halloy::view)
        .theme(Halloy::theme)
        .scale_factor(Halloy::scale_factor)
        .subscription(Halloy::subscription)
        .settings(settings(&config_load))
        .run_with(move || Halloy::new(config_load.clone(), destination.clone()))
    {
        log::error!("{}", error.to_string());
        Err(error)
    } else {
        Ok(())
    }
}

fn settings(config_load: &Result<Config, config::Error>) -> iced::Settings {
    let default_text_size = config_load
        .as_ref()
        .ok()
        .and_then(|config| config.font.size)
        .map(f32::from)
        .unwrap_or(theme::TEXT_SIZE);

    iced::Settings {
        default_font: font::MONO.clone().into(),
        default_text_size: default_text_size.into(),
        id: None,
        antialiasing: false,
        fonts: font::load(),
    }
}

struct Halloy {
    version: Version,
    screen: Screen,
    theme: Theme,
    config: Config,
    clients: data::client::Map,
    servers: server::Map,
    modal: Option<Modal>,
    main_window: Window,
}

impl Halloy {
    pub fn load_from_state(
        main_window: window::Id,
        config_load: Result<Config, config::Error>,
    ) -> (Halloy, Task<Message>) {
        let main_window = Window::new(main_window);

        let load_dashboard = |config| match data::Dashboard::load() {
            Ok(dashboard) => screen::Dashboard::restore(dashboard, config, &main_window),
            Err(error) => {
                log::warn!("failed to load dashboard: {error}");

                screen::Dashboard::empty(config)
            }
        };

        let (screen, config, command) = match config_load {
            Ok(config) => {
                let (screen, command) = load_dashboard(&config);

                (
                    Screen::Dashboard(screen),
                    config,
                    command.map(Message::Dashboard),
                )
            }
            Err(error) => match &error {
                config::Error::Parse(_) | config::Error::LoadSounds(_) => (
                    Screen::Help(screen::Help::new(error)),
                    Config::default(),
                    Task::none(),
                ),
                _ => {
                    // If we have a YAML file, but end up in this arm
                    // it means the user tried to load Halloy with a YAML configuration, but it expected TOML.
                    if config::has_yaml_config() {
                        (
                            Screen::Migration(screen::Migration::new()),
                            Config::default(),
                            Task::none(),
                        )
                    } else {
                        // Otherwise, show regular welcome screen for new users.
                        (
                            Screen::Welcome(screen::Welcome::new()),
                            Config::default(),
                            Task::none(),
                        )
                    }
                }
            },
        };

        (
            Halloy {
                version: Version::new(),
                screen,
                theme: config.themes.default.clone().into(),
                clients: Default::default(),
                servers: config.servers.clone(),
                config,
                modal: None,
                main_window,
            },
            command,
        )
    }
}

pub enum Screen {
    Dashboard(screen::Dashboard),
    Help(screen::Help),
    Welcome(screen::Welcome),
    Migration(screen::Migration),
}

#[derive(Debug)]
pub enum Message {
    Dashboard(dashboard::Message),
    Stream(stream::Update),
    Help(help::Message),
    Welcome(welcome::Message),
    Migration(migration::Message),
    Event(window::Id, Event),
    Tick(Instant),
    Version(Option<String>),
    Modal(modal::Message),
    RouteReceived(String),
    Window(window::Id, window::Event),
    WindowSettingsSaved(Result<(), window::Error>),
}

impl Halloy {
    fn new(
        config_load: Result<Config, config::Error>,
        url_received: Option<data::Url>,
    ) -> (Halloy, Task<Message>) {
        let (main_window, open_main_window) = window::open(window::Settings {
            size: window::default_size(),
            position: window::Position::Default,
            min_size: Some(window::MIN_SIZE),
            exit_on_close_request: false,
            ..window::settings()
        });

        let (mut halloy, command) = Halloy::load_from_state(main_window, config_load);
        let latest_remote_version =
            Task::perform(version::latest_remote_version(), Message::Version);

        let mut commands = vec![
            open_main_window.then(|_| Task::none()),
            command,
            latest_remote_version,
        ];

        if let Some(url) = url_received {
            commands.push(halloy.handle_url(url));
        }

        (halloy, Task::batch(commands))
    }

    fn handle_url(&mut self, url: Url) -> Task<Message> {
        match url {
            data::Url::ServerConnect {
                url,
                server,
                config,
            } => {
                self.modal = Some(Modal::ServerConnect {
                    url,
                    server,
                    config,
                });
            }
            data::Url::Theme { colors, .. } => {
                if let Screen::Dashboard(dashboard) = &mut self.screen {
                    return dashboard
                        .preview_theme_in_editor(colors, &self.main_window, &mut self.theme)
                        .map(Message::Dashboard);
                }
            }
            data::Url::Unknown(url) => {
                log::warn!("Received unknown url: {url}");
            }
        }

        Task::none()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Dashboard(message) => {
                let Screen::Dashboard(dashboard) = &mut self.screen else {
                    return Task::none();
                };

                let (command, event) = dashboard.update(
                    message,
                    &mut self.clients,
                    &mut self.theme,
                    &self.version,
                    &self.config,
                    &self.main_window,
                );

                // Retrack after dashboard state changes
                let track = dashboard.track();

                if let Some(event) = event {
                    match event {
                        dashboard::Event::ReloadConfiguration => match Config::load() {
                            Ok(updated) => {
                                let removed_servers = self
                                    .servers
                                    .keys()
                                    .filter(|server| !updated.servers.contains(server))
                                    .cloned()
                                    .collect::<Vec<_>>();

                                self.servers = updated.servers.clone();
                                self.theme = updated.themes.default.clone().into();
                                self.config = updated;

                                for server in removed_servers {
                                    self.clients.quit(&server, None);
                                }
                            }
                            Err(error) => {
                                self.modal = Some(Modal::ReloadConfigurationError(error));
                            }
                        },
                        dashboard::Event::ReloadThemes => {
                            if let Ok(updated) = Config::load() {
                                self.config.themes = updated.themes;
                            }
                        }
                        dashboard::Event::QuitServer(server) => {
                            self.clients.quit(&server, None);
                        }
                    }
                }

                Task::batch(vec![
                    command.map(Message::Dashboard),
                    track.map(Message::Dashboard),
                ])
            }
            Message::Version(remote) => {
                // Set latest known remote version
                self.version.remote = remote;

                Task::none()
            }
            Message::Help(message) => {
                let Screen::Help(help) = &mut self.screen else {
                    return Task::none();
                };

                if let Some(event) = help.update(message) {
                    match event {
                        help::Event::RefreshConfiguration => {
                            let (halloy, command) =
                                Halloy::load_from_state(self.main_window.id, Config::load());
                            *self = halloy;

                            return command;
                        }
                    }
                }

                Task::none()
            }
            Message::Welcome(message) => {
                let Screen::Welcome(welcome) = &mut self.screen else {
                    return Task::none();
                };

                if let Some(event) = welcome.update(message) {
                    match event {
                        welcome::Event::RefreshConfiguration => {
                            let (halloy, command) =
                                Halloy::load_from_state(self.main_window.id, Config::load());
                            *self = halloy;

                            return command;
                        }
                    }
                }

                Task::none()
            }
            Message::Migration(message) => {
                let Screen::Migration(migration) = &mut self.screen else {
                    return Task::none();
                };

                if let Some(event) = migration.update(message) {
                    match event {
                        migration::Event::RefreshConfiguration => {
                            let (halloy, command) =
                                Halloy::load_from_state(self.main_window.id, Config::load());
                            *self = halloy;

                            return command;
                        }
                    }
                }

                Task::none()
            }
            Message::Stream(update) => match update {
                stream::Update::Disconnected {
                    server,
                    is_initial,
                    error,
                    sent_time,
                } => {
                    self.clients.disconnected(server.clone());

                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Task::none();
                    };

                    if is_initial {
                        // Intial is sent when first trying to connect
                        dashboard.broadcast_connecting(&server, &self.config, sent_time);
                    } else {
                        notification::disconnected(&self.config.notifications, &server);

                        dashboard.broadcast_disconnected(&server, error, &self.config, sent_time);
                    }

                    Task::none()
                }
                stream::Update::Connected {
                    server,
                    client: connection,
                    is_initial,
                    sent_time,
                } => {
                    self.clients.ready(server.clone(), connection);

                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Task::none();
                    };

                    if is_initial {
                        notification::connected(&self.config.notifications, &server);

                        dashboard.broadcast_connected(&server, &self.config, sent_time);
                    } else {
                        notification::reconnected(&self.config.notifications, &server);

                        dashboard.broadcast_reconnected(&server, &self.config, sent_time);
                    }

                    Task::none()
                }
                stream::Update::ConnectionFailed {
                    server,
                    error,
                    sent_time,
                } => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Task::none();
                    };

                    dashboard.broadcast_connection_failed(&server, error, &self.config, sent_time);

                    Task::none()
                }
                stream::Update::MessagesReceived(server, messages) => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Task::none();
                    };

                    let commands = messages
                        .into_iter()
                        .flat_map(|message| {
                            let mut commands = vec![];

                            let events = self.clients.receive(&server, message);

                            for event in events {
                                // Resolve a user using client state which stores attributes
                                let resolve_user_attributes = |user: &User, channel: &str| {
                                    self.clients.resolve_user_attributes(&server, channel, user).cloned()
                                };

                                match event {
                                    data::client::Event::Single(encoded, our_nick) => {
                                        if let Some(message) =
                                            data::Message::received(encoded, our_nick, &self.config, resolve_user_attributes)
                                        {
                                            dashboard.record_message(&server, message);
                                        }
                                    }
                                    data::client::Event::WithTarget(encoded, our_nick, target) => {
                                        if let Some(message) =
                                            data::Message::received(encoded, our_nick, &self.config, resolve_user_attributes)
                                        {
                                            dashboard.record_message(&server, message.with_target(target));
                                        }
                                    }
                                    data::client::Event::Broadcast(broadcast) => match broadcast {
                                        data::client::Broadcast::Quit {
                                            user,
                                            comment,
                                            channels,
                                            sent_time,
                                        } => {
                                            dashboard.broadcast_quit(
                                                &server,
                                                user,
                                                comment,
                                                channels,
                                                &self.config,
                                                sent_time
                                            );
                                        }
                                        data::client::Broadcast::Nickname {
                                            old_user,
                                            new_nick,
                                            ourself,
                                            channels,
                                            sent_time,
                                        } => {
                                            let old_nick = old_user.nickname();

                                            dashboard.broadcast_nickname(
                                                &server,
                                                old_nick.to_owned(),
                                                new_nick,
                                                ourself,
                                                channels,
                                                &self.config,
                                                sent_time,
                                            );
                                        }
                                        data::client::Broadcast::Invite {
                                            inviter,
                                            channel,
                                            user_channels,
                                            sent_time,
                                        } => {
                                            let inviter = inviter.nickname();

                                            dashboard.broadcast_invite(
                                                &server,
                                                inviter.to_owned(),
                                                channel,
                                                user_channels,
                                                &self.config,
                                                sent_time,
                                            );
                                        }
                                        data::client::Broadcast::ChangeHost {
                                            old_user,
                                            new_username,
                                            new_hostname,
                                            ourself,
                                            channels,
                                            sent_time,
                                        } => {
                                            dashboard.broadcast_change_host(
                                                &server,
                                                old_user,
                                                new_username,
                                                new_hostname,
                                                ourself,
                                                channels,
                                                &self.config,
                                                sent_time,
                                            );
                                        }
                                    },
                                    data::client::Event::Notification(encoded, our_nick, notification) => {
                                        if let Some(message) =
                                            data::Message::received(encoded, our_nick, &self.config, resolve_user_attributes)
                                        {
                                            dashboard.record_message(&server, message);
                                        }

                                        match notification {
                                            data::client::Notification::DirectMessage(user) => {
                                                // only send notification if query has unread
                                                // or if window is not focused
                                                if dashboard.history().has_unread(
                                                    &server,
                                                    &history::Kind::Query(
                                                        user.nickname().to_owned(),
                                                    ),
                                                ) || !self.main_window.focused
                                                {
                                                    notification::direct_message(
                                                        &self.config.notifications,
                                                        user.nickname(),
                                                    );
                                                }
                                            }
                                            data::client::Notification::Highlight(
                                                user,
                                                channel,
                                            ) => {
                                                notification::highlight(
                                                    &self.config.notifications,
                                                    user.nickname(),
                                                    channel,
                                                );
                                            }
                                            data::client::Notification::MonitoredOnline(
                                                targets,
                                            ) => {
                                                targets.into_iter().for_each(|target| {
                                                    notification::monitored_online(
                                                        &self.config.notifications,
                                                        target.nickname().to_owned(),
                                                        server.clone(),
                                                    );
                                                });
                                            }
                                            data::client::Notification::MonitoredOffline(
                                                targets,
                                            ) => {
                                                targets.into_iter().for_each(|target| {
                                                    notification::monitored_offline(
                                                        &self.config.notifications,
                                                        target,
                                                        server.clone(),
                                                    );
                                                });
                                            }
                                        }
                                    }
                                    data::client::Event::FileTransferRequest(request) => {
                                        if let Some(command) = dashboard.receive_file_transfer(
                                            &server,
                                            request,
                                            &self.config,
                                        ) {
                                            commands.push(command.map(Message::Dashboard));
                                        }
                                    }
                                    data::client::Event::ChatHistoryRequest(subcommand) => {
                                        self.clients.send_chathistory_request(
                                            &server,
                                            subcommand,
                                        );
                                    }
                                    data::client::Event::ChatHistoryRequestFromHistory(
                                        history_request,
                                        target,
                                        message_reference_types,
                                    ) => {
                                        let subcommand = match history_request {
                                            data::client::HistoryRequest::Recent(
                                                join_server_time,
                                            ) => {
                                                dashboard.load_history_now(server.clone(), &target);

                                                ChatHistorySubcommand::Latest(
                                                    target.clone(),
                                                    dashboard.get_latest_message_reference(
                                                        &server,
                                                        &target,
                                                        &message_reference_types,
                                                        join_server_time,
                                                    ),
                                                    self.clients.get_server_chathistory_limit(&server),
                                                )
                                            }
                                            data::client::HistoryRequest::Older => {
                                                let message_reference = dashboard.get_oldest_message_reference(
                                                        &server,
                                                        &target,
                                                        &message_reference_types,
                                                    );

                                                if matches!(message_reference, MessageReference::None) {
                                                    ChatHistorySubcommand::Latest(
                                                        target.clone(),
                                                        message_reference,
                                                        self.clients.get_server_chathistory_limit(&server),
                                                    )
                                                } else {
                                                    ChatHistorySubcommand::Before(
                                                        target.clone(),
                                                        message_reference,
                                                        self.clients.get_server_chathistory_limit(&server),
                                                    )
                                                }
                                            }
                                        };

                                        self.clients.send_chathistory_request(
                                            &server,
                                            subcommand,
                                        );
                                    }
                                    data::client::Event::ChatHistoryRequestReceived(
                                        subcommand,
                                        batch_len,
                                    ) => {
                                        match &subcommand {
                                            ChatHistorySubcommand::Latest(target, message_reference, _) => {
                                                log::debug!(
                                                    "[{}] received latest {} messages in {} since {}",
                                                    server,
                                                    batch_len,
                                                    target,
                                                    message_reference,
                                                );
                                            }
                                            ChatHistorySubcommand::Before(target, message_reference, _) => {
                                                log::debug!(
                                                    "[{}] received {} messages in {} before {}",
                                                    server,
                                                    batch_len,
                                                    target,
                                                    message_reference,
                                                );
                                            }
                                            ChatHistorySubcommand::Between(
                                                target,
                                                start_message_reference,
                                                end_message_reference,
                                                _,
                                            ) => {
                                                log::debug!(
                                                    "[{}] received {} messages in {} between {} and {}",
                                                    server,
                                                    batch_len,
                                                    target,
                                                    start_message_reference,
                                                    end_message_reference,
                                                );
                                            }
                                        }

                                        if let Some(target) = subcommand.target() {
                                            if !dashboard.is_open(server.clone(), target) {
                                                dashboard.make_history_partial_now(
                                                    server.clone(),
                                                    target,
                                                    if let ChatHistorySubcommand::Latest(_, message_reference, _) = &subcommand {
                                                        Some(message_reference.clone())
                                                    } else {
                                                        None
                                                    },
                                                );
                                            }

                                            self.clients.clear_chathistory_request(&server, target);
                                        }
                                    }
                                }
                            }

                            commands
                        })
                        .collect::<Vec<_>>();

                    // Must be called after receiving message batches to ensure
                    // user & channel lists are in sync
                    self.clients.sync(&server);

                    Task::batch(commands)
                }
                stream::Update::Quit(server, reason) => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Task::none();
                    };

                    self.servers.remove(&server);

                    if let Some(client) = self.clients.remove(&server) {
                        let user = client.nickname().to_owned().into();

                        let channels = client.channels().to_vec();

                        dashboard.broadcast_quit(
                            &server,
                            user,
                            reason,
                            channels,
                            &self.config,
                            Utc::now(),
                        );
                    }

                    Task::none()
                }
            },
            Message::Event(window, event) => {
                // Events only enabled for main window
                if window == self.main_window.id {
                    if let Screen::Dashboard(dashboard) = &mut self.screen {
                        return dashboard
                            .handle_event(
                                event,
                                &mut self.clients,
                                &self.version,
                                &self.config,
                                &mut self.theme,
                                &self.main_window,
                            )
                            .map(Message::Dashboard);
                    }
                }

                Task::none()
            }
            Message::Tick(now) => {
                self.clients.tick(now);

                if let Screen::Dashboard(dashboard) = &mut self.screen {
                    dashboard.tick(now).map(Message::Dashboard)
                } else {
                    Task::none()
                }
            }
            Message::Modal(message) => {
                let Some(modal) = &mut self.modal else {
                    return Task::none();
                };

                if let Some(event) = modal.update(message) {
                    match event {
                        modal::Event::CloseModal => {
                            self.modal = None;
                        }
                        modal::Event::AcceptNewServer => {
                            if let Some(Modal::ServerConnect { server, config, .. }) =
                                self.modal.take()
                            {
                                let existing_entry = self.servers.entries().find(|entry| {
                                    entry.server == server || entry.config.server == config.server
                                });

                                // If server already exists, we only want to join the new channels
                                if let Some(entry) = existing_entry {
                                    self.clients.join(&entry.server, &config.channels);
                                } else {
                                    self.servers.insert(server, config);
                                }
                            }
                        }
                    }
                }

                Task::none()
            }
            Message::RouteReceived(route) => {
                log::info!("RouteRecived: {:?}", route);

                if let Ok(url) = route.parse() {
                    return self.handle_url(url);
                };

                Task::none()
            }
            Message::Window(id, event) => {
                if id == self.main_window.id {
                    match event {
                        window::Event::Moved(position) => {
                            self.main_window.position = Some(position)
                        }
                        window::Event::Resized(size) => self.main_window.size = size,
                        window::Event::Focused => self.main_window.focused = true,
                        window::Event::Unfocused => self.main_window.focused = false,
                        window::Event::Opened { position, size } => {
                            self.main_window.opened(position, size)
                        }
                        window::Event::CloseRequested => {
                            if let Screen::Dashboard(dashboard) = &mut self.screen {
                                return dashboard.exit().then(|_| iced::exit());
                            } else {
                                return iced::exit();
                            }
                        }
                    }

                    Task::perform(
                        data::Window::from(self.main_window).save(),
                        Message::WindowSettingsSaved,
                    )
                } else if let Screen::Dashboard(dashboard) = &mut self.screen {
                    dashboard
                        .handle_window_event(id, event, &mut self.theme)
                        .map(Message::Dashboard)
                } else {
                    Task::none()
                }
            }
            Message::WindowSettingsSaved(result) => {
                if let Err(err) = result {
                    log::error!("window settings failed to save: {:?}", err)
                }

                Task::none()
            }
        }
    }

    fn view(&self, id: window::Id) -> Element<Message> {
        let content = if id == self.main_window.id {
            let now = Instant::now();

            let screen = match &self.screen {
                Screen::Dashboard(dashboard) => dashboard
                    .view(
                        now,
                        &self.clients,
                        &self.version,
                        &self.config,
                        &self.theme,
                        &self.main_window,
                    )
                    .map(Message::Dashboard),
                Screen::Help(help) => help.view().map(Message::Help),
                Screen::Welcome(welcome) => welcome.view().map(Message::Welcome),
                Screen::Migration(migration) => migration.view().map(Message::Migration),
            };

            let content = container(screen)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::container::general);

            if let (Some(modal), Screen::Dashboard(_)) = (&self.modal, &self.screen) {
                widget::modal(content, modal.view().map(Message::Modal), || {
                    Message::Modal(modal::Message::Cancel)
                })
            } else {
                // Align `content` into same view tree shape as `modal`
                // to prevent diff from firing when displaying modal
                column![content].into()
            }
        } else if let Screen::Dashboard(dashboard) = &self.screen {
            dashboard
                .view_window(
                    id,
                    &self.clients,
                    &self.config,
                    &self.theme,
                    &self.main_window,
                )
                .map(Message::Dashboard)
        } else {
            column![].into()
        };

        // The height margin varies across different operating systems due to design differences.
        // For instance, on macOS, the menubar is hidden, resulting in a need for additional padding to accommodate the
        // space occupied by the traffic light buttons.
        let height_margin = if cfg!(target_os = "macos") { 20 } else { 0 };

        container(content)
            .padding(padding::top(height_margin))
            .style(theme::container::general)
            .into()
    }

    fn theme(&self, _window: window::Id) -> Theme {
        self.theme.clone()
    }

    fn scale_factor(&self, _window: window::Id) -> f64 {
        self.config.scale_factor.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(Duration::from_secs(1)).map(Message::Tick);

        let streams = Subscription::batch(
            self.servers
                .entries()
                .map(|entry| stream::run(entry, self.config.proxy.clone())),
        )
        .map(Message::Stream);

        Subscription::batch(vec![
            url::listen().map(Message::RouteReceived),
            events().map(|(window, event)| Message::Event(window, event)),
            window::events().map(|(window, event)| Message::Window(window, event)),
            tick,
            streams,
        ])
    }
}
