#![allow(clippy::large_enum_variant, clippy::too_many_arguments)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod appearance;
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
mod url;
mod widget;
mod window;

use std::collections::HashSet;
use std::time::{Duration, Instant};
use std::{env, mem};

use appearance::{theme, Theme};
use chrono::Utc;
use data::config::{self, Config};
use data::history::{self, manager::Broadcast};
use data::version::Version;
use data::{environment, server, version, Server, Url, User};
use iced::widget::{column, container};
use iced::{padding, Length, Subscription, Task};
use screen::{dashboard, help, migration, welcome};
use tokio::runtime;
use tokio_stream::wrappers::ReceiverStream;

use self::event::{events, Event};
use self::modal::Modal;
use self::widget::Element;
use self::window::Window;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let is_debug = cfg!(debug_assertions);

    // Prepare notifications.
    notification::prepare();

    let log_stream = logger::setup(is_debug).expect("setup logging");
    log::info!("halloy {} has started", environment::formatted_version());
    log::info!("config dir: {:?}", environment::config_dir());
    log::info!("data dir: {:?}", environment::data_dir());

    // spin up a single-threaded tokio runtime to run the config loading task to completion
    // we don't want to wrap our whole program with a runtime since iced starts its own.
    let config_load = {
        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        rt.block_on(Config::load())
    };

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

    iced::daemon("Halloy", Halloy::update, Halloy::view)
        .theme(Halloy::theme)
        .scale_factor(Halloy::scale_factor)
        .subscription(Halloy::subscription)
        .settings(settings(&config_load))
        .run_with(move || Halloy::new(config_load, destination, log_stream))
        .inspect_err(|err| log::error!("{}", err))?;

    Ok(())
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

fn handle_irc_error(e: anyhow::Error) {
    log::error!("IRC error: {:?}", e);
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
    pending_logs: Vec<data::log::Record>,
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
            // If we have a YAML file, but end up in this arm
            // it means the user tried to load Halloy with a YAML configuration, but it expected TOML.
            Err(config::Error::ConfigMissing {
                has_yaml_config: true,
            }) => (
                Screen::Migration(screen::Migration::new()),
                Config::default(),
                Task::none(),
            ),
            // Show regular welcome screen for new users.
            Err(config::Error::ConfigMissing {
                has_yaml_config: false,
            }) => (
                Screen::Welcome(screen::Welcome::new()),
                Config::default(),
                Task::none(),
            ),
            Err(error) => (
                Screen::Help(screen::Help::new(error)),
                Config::default(),
                Task::none(),
            ),
        };

        (
            Halloy {
                version: Version::new(),
                screen,
                theme: appearance::theme(&config.appearance.selected).into(),
                clients: Default::default(),
                servers: config.servers.clone(),
                config,
                modal: None,
                main_window,
                pending_logs: vec![],
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
    Exit { pending_exit: HashSet<Server> },
}

#[derive(Debug)]
pub enum Message {
    AppearanceReloaded(data::appearance::Appearance),
    ScreenConfigReloaded(Result<Config, config::Error>),
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
    AppearanceChange(appearance::Mode),
    Window(window::Id, window::Event),
    WindowSettingsSaved(Result<(), window::Error>),
    Logging(Vec<logger::Record>),
}

impl Halloy {
    fn new(
        config_load: Result<Config, config::Error>,
        url_received: Option<data::Url>,
        log_stream: ReceiverStream<Vec<logger::Record>>,
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
            Task::stream(log_stream).map(Message::Logging),
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
            Message::AppearanceReloaded(appearance) => {
                self.config.appearance = appearance;
                Task::none()
            }
            Message::ScreenConfigReloaded(updated) => {
                let (halloy, command) = Halloy::load_from_state(self.main_window.id, updated);
                *self = halloy;
                command
            }
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

                let event_task = match event {
                    Some(dashboard::Event::ConfigReloaded(config)) => {
                        match config {
                            Ok(updated) => {
                                let removed_servers = self
                                    .servers
                                    .keys()
                                    .filter(|server| !updated.servers.contains(server))
                                    .cloned()
                                    .collect::<Vec<_>>();

                                self.servers = updated.servers.clone();
                                self.theme = appearance::theme(&updated.appearance.selected).into();
                                self.config = updated;

                                for server in removed_servers {
                                    self.clients.quit(&server, None);
                                }
                            }
                            Err(error) => {
                                self.modal = Some(Modal::ReloadConfigurationError(error));
                            }
                        };
                        Task::none()
                    }
                    Some(dashboard::Event::ReloadThemes) => Task::future(Config::load())
                        .and_then(|config| Task::done(config.appearance))
                        .map(Message::AppearanceReloaded),
                    Some(dashboard::Event::QuitServer(server)) => {
                        self.clients.quit(&server, None);
                        Task::none()
                    }
                    Some(dashboard::Event::IrcError(e)) => {
                        handle_irc_error(e);
                        Task::none()
                    }
                    Some(dashboard::Event::Exit) => {
                        let pending_exit = self.clients.exit();

                        if pending_exit.is_empty() {
                            iced::exit()
                        } else {
                            self.screen = Screen::Exit { pending_exit };
                            Task::none()
                        }
                    }
                    None => Task::none(),
                };

                Task::batch(vec![
                    event_task,
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

                match help.update(message) {
                    Some(help::Event::RefreshConfiguration) => {
                        Task::perform(Config::load(), Message::ScreenConfigReloaded)
                    }
                    None => Task::none(),
                }
            }
            Message::Welcome(message) => {
                let Screen::Welcome(welcome) = &mut self.screen else {
                    return Task::none();
                };

                match welcome.update(message) {
                    Some(welcome::Event::RefreshConfiguration) => {
                        Task::perform(Config::load(), Message::ScreenConfigReloaded)
                    }
                    None => Task::none(),
                }
            }
            Message::Migration(message) => {
                let Screen::Migration(migration) = &mut self.screen else {
                    return Task::none();
                };

                match migration.update(message) {
                    Some(migration::Event::RefreshConfiguration) => {
                        Task::perform(Config::load(), Message::ScreenConfigReloaded)
                    }
                    None => Task::none(),
                }
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
                        dashboard
                            .broadcast(&server, &self.config, sent_time, Broadcast::Connecting)
                            .map(Message::Dashboard)
                    } else {
                        notification::disconnected(&self.config.notifications, &server);

                        dashboard
                            .broadcast(
                                &server,
                                &self.config,
                                sent_time,
                                Broadcast::Disconnected { error },
                            )
                            .map(Message::Dashboard)
                    }
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

                        dashboard
                            .broadcast(&server, &self.config, sent_time, Broadcast::Connected)
                            .map(Message::Dashboard)
                    } else {
                        notification::reconnected(&self.config.notifications, &server);

                        dashboard
                            .broadcast(&server, &self.config, sent_time, Broadcast::Reconnected)
                            .map(Message::Dashboard)
                    }
                }
                stream::Update::ConnectionFailed {
                    server,
                    error,
                    sent_time,
                } => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Task::none();
                    };

                    dashboard
                        .broadcast(
                            &server,
                            &self.config,
                            sent_time,
                            Broadcast::ConnectionFailed { error },
                        )
                        .map(Message::Dashboard)
                }
                stream::Update::MessagesReceived(server, messages) => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Task::none();
                    };

                    let commands = messages
                        .into_iter()
                        .flat_map(|message| {
                            let events = match self.clients.receive(&server, message) {
                                Ok(events) => events,
                                Err(e) => {
                                    handle_irc_error(e);
                                    return vec![];
                                }
                            };

                            let mut commands = vec![];

                            for event in events {
                                // Resolve a user using client state which stores attributes
                                let resolve_user_attributes = |user: &User, channel: &str| {
                                    self.clients
                                        .resolve_user_attributes(&server, channel, user)
                                        .cloned()
                                };

                                let channel_users = |channel: &str| -> &[User] {
                                    self.clients.get_channel_users(&server, channel)
                                };

                                let chantypes = self.clients.get_chantypes(&server);
                                let statusmsg = self.clients.get_statusmsg(&server);

                                match event {
                                    data::client::Event::Single(encoded, our_nick) => {
                                        if let Some(message) = data::Message::received(
                                            encoded,
                                            our_nick,
                                            &self.config,
                                            resolve_user_attributes,
                                            channel_users,
                                            chantypes,
                                            statusmsg,
                                        ) {
                                            commands.push(
                                                dashboard
                                                    .record_message(&server, message)
                                                    .map(Message::Dashboard),
                                            );
                                        }
                                    }
                                    data::client::Event::WithTarget(encoded, our_nick, target) => {
                                        if let Some(message) = data::Message::received(
                                            encoded,
                                            our_nick,
                                            &self.config,
                                            resolve_user_attributes,
                                            channel_users,
                                            chantypes,
                                            statusmsg,
                                        ) {
                                            commands.push(
                                                dashboard
                                                    .record_message(
                                                        &server,
                                                        message.with_target(target),
                                                    )
                                                    .map(Message::Dashboard),
                                            );
                                        }
                                    }
                                    data::client::Event::Broadcast(broadcast) => match broadcast {
                                        data::client::Broadcast::Quit {
                                            user,
                                            comment,
                                            channels,
                                            sent_time,
                                        } => commands.push(
                                            dashboard
                                                .broadcast(
                                                    &server,
                                                    &self.config,
                                                    sent_time,
                                                    Broadcast::Quit {
                                                        user,
                                                        comment,
                                                        user_channels: channels,
                                                    },
                                                )
                                                .map(Message::Dashboard),
                                        ),
                                        data::client::Broadcast::Nickname {
                                            old_user,
                                            new_nick,
                                            ourself,
                                            channels,
                                            sent_time,
                                        } => {
                                            let old_nick = old_user.nickname();

                                            commands.push(
                                                dashboard
                                                    .broadcast(
                                                        &server,
                                                        &self.config,
                                                        sent_time,
                                                        Broadcast::Nickname {
                                                            old_nick: old_nick.to_owned(),
                                                            new_nick,
                                                            ourself,
                                                            user_channels: channels,
                                                        },
                                                    )
                                                    .map(Message::Dashboard),
                                            );
                                        }
                                        data::client::Broadcast::Invite {
                                            inviter,
                                            channel,
                                            user_channels,
                                            sent_time,
                                        } => {
                                            let inviter = inviter.nickname();

                                            commands.push(
                                                dashboard
                                                    .broadcast(
                                                        &server,
                                                        &self.config,
                                                        sent_time,
                                                        Broadcast::Invite {
                                                            inviter: inviter.to_owned(),
                                                            channel,
                                                            user_channels,
                                                        },
                                                    )
                                                    .map(Message::Dashboard),
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
                                            commands.push(
                                                dashboard
                                                    .broadcast(
                                                        &server,
                                                        &self.config,
                                                        sent_time,
                                                        Broadcast::ChangeHost {
                                                            old_user,
                                                            new_username,
                                                            new_hostname,
                                                            ourself,
                                                            user_channels: channels,
                                                        },
                                                    )
                                                    .map(Message::Dashboard),
                                            );
                                        }
                                    },
                                    data::client::Event::Notification(
                                        encoded,
                                        our_nick,
                                        notification,
                                    ) => {
                                        if let Some(message) = data::Message::received(
                                            encoded,
                                            our_nick,
                                            &self.config,
                                            resolve_user_attributes,
                                            channel_users,
                                            chantypes,
                                            statusmsg,
                                        ) {
                                            commands.push(
                                                dashboard
                                                    .record_message(&server, message.clone())
                                                    .map(Message::Dashboard),
                                            );

                                            if matches!(
                                                notification,
                                                data::client::Notification::Highlight { .. }
                                            ) {
                                                commands.extend(
                                                    message.into_highlight(server.clone()).map(
                                                        |message| {
                                                            dashboard
                                                                .record_highlight(message)
                                                                .map(Message::Dashboard)
                                                        },
                                                    ),
                                                );
                                            }
                                        }

                                        match notification {
                                            data::client::Notification::DirectMessage(user) => {
                                                // only send notification if query has unread
                                                // or if window is not focused
                                                if dashboard.history().has_unread(
                                                    &history::Kind::Query(
                                                        server.clone(),
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
                                            data::client::Notification::Highlight {
                                                enabled,
                                                user,
                                                channel,
                                            } => {
                                                if enabled {
                                                    notification::highlight(
                                                        &self.config.notifications,
                                                        user.nickname(),
                                                        channel,
                                                    );
                                                }
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
                                    data::client::Event::UpdateReadMarker(target, read_marker) => {
                                        commands.push(
                                            dashboard
                                                .update_read_marker(
                                                    history::Kind::from_target(
                                                        server.clone(),
                                                        target,
                                                        chantypes,
                                                    ),
                                                    read_marker,
                                                )
                                                .map(Message::Dashboard),
                                        );
                                    }
                                    data::client::Event::JoinedChannel(channel, server_time) => {
                                        let command = dashboard
                                            .load_metadata(
                                                &self.clients,
                                                server.clone(),
                                                channel.clone(),
                                                server_time,
                                            )
                                            .map(Message::Dashboard);

                                        commands.push(command);
                                    }
                                    data::client::Event::ChatHistoryAcknowledged(server_time) => {
                                        if let Some(command) = dashboard
                                            .load_chathistory_targets_timestamp(
                                                &self.clients,
                                                &server,
                                                server_time,
                                            )
                                            .map(|command| command.map(Message::Dashboard))
                                        {
                                            commands.push(command);
                                        }
                                    }
                                    data::client::Event::ChatHistoryTargetReceived(
                                        target,
                                        server_time,
                                    ) => {
                                        let command = dashboard
                                            .load_metadata(
                                                &self.clients,
                                                server.clone(),
                                                target.clone(),
                                                server_time,
                                            )
                                            .map(Message::Dashboard);

                                        commands.push(command);
                                    }
                                    data::client::Event::ChatHistoryTargetsReceived(
                                        server_time,
                                    ) => {
                                        if let Some(command) = dashboard
                                            .overwrite_chathistory_targets_timestamp(
                                                &self.clients,
                                                &server,
                                                server_time,
                                            )
                                            .map(|command| command.map(Message::Dashboard))
                                        {
                                            commands.push(command);
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
                stream::Update::Quit(server, reason) => match &mut self.screen {
                    Screen::Dashboard(dashboard) => {
                        self.servers.remove(&server);

                        if let Some(client) = self.clients.remove(&server) {
                            let user = client.nickname().to_owned().into();

                            let channels = client.channels().to_vec();

                            dashboard
                                .broadcast(
                                    &server,
                                    &self.config,
                                    Utc::now(),
                                    Broadcast::Quit {
                                        user,
                                        comment: reason,
                                        user_channels: channels,
                                    },
                                )
                                .map(Message::Dashboard)
                        } else {
                            Task::none()
                        }
                    }
                    Screen::Exit { pending_exit } => {
                        pending_exit.remove(&server);

                        if pending_exit.is_empty() {
                            iced::exit()
                        } else {
                            Task::none()
                        }
                    }
                    _ => Task::none(),
                },
            },
            Message::Event(window, event) => {
                if let Screen::Dashboard(dashboard) = &mut self.screen {
                    return dashboard
                        .handle_event(
                            window,
                            event,
                            &mut self.clients,
                            &self.version,
                            &self.config,
                            &mut self.theme,
                            &self.main_window,
                        )
                        .map(Message::Dashboard);
                }

                Task::none()
            }
            Message::Tick(now) => {
                if let Err(e) = self.clients.tick(now) {
                    handle_irc_error(e);
                    Task::none()
                } else if let Screen::Dashboard(dashboard) = &mut self.screen {
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
                                return dashboard.exit().map(Message::Dashboard);
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
            Message::AppearanceChange(mode) => {
                if let data::appearance::Selected::Dynamic { light, dark } =
                    &self.config.appearance.selected
                {
                    self.theme = match mode {
                        appearance::Mode::Dark => dark.clone().into(),
                        appearance::Mode::Light => light.clone().into(),
                    }
                }

                Task::none()
            }
            Message::Logging(mut records) => {
                let Screen::Dashboard(dashboard) = &mut self.screen else {
                    self.pending_logs.extend(records);

                    return Task::none();
                };

                // We've moved from non-dashboard screen to dashboard, prepend records
                if !self.pending_logs.is_empty() {
                    records = mem::take(&mut self.pending_logs)
                        .into_iter()
                        .chain(records)
                        .collect();
                }

                Task::batch(
                    records
                        .into_iter()
                        .map(|record| dashboard.record_log(record)),
                )
                .map(Message::Dashboard)
            }
        }
    }

    fn view(&self, id: window::Id) -> Element<Message> {
        let content = if id == self.main_window.id {
            let screen = match &self.screen {
                Screen::Dashboard(dashboard) => dashboard
                    .view(
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
                Screen::Exit { .. } => column![].into(),
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
            // Enable once dark_light has a proper way to detect appereance changes without spiking CPU.
            // See: https://github.com/frewsxcv/rust-dark-light/issues/47
            // appearance::subscription().map(Message::AppearanceChange),
            tick,
            streams,
        ])
    }
}
