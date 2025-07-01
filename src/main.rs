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
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::{env, mem};

use appearance::{Theme, theme};
use chrono::Utc;
use data::config::{self, Config};
use data::history::manager::Broadcast;
use data::message::Decoded;
use data::target::{self, Target};
use data::version::Version;
use data::{
    Notification, Server, Url, User, client, environment, history, message,
    server, version,
};
use iced::widget::{column, container};
use iced::{Length, Subscription, Task, padding};
use screen::{dashboard, help, migration, welcome};
use tokio::runtime;
use tokio_stream::wrappers::ReceiverStream;

use self::event::{Event, events};
use self::modal::Modal;
use self::notification::Notifications;
use self::widget::Element;
use self::window::Window;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();
    args.next();

    let version = args.next().is_some_and(|s| s == "--version" || s == "-V");

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
    let (config_load, window_load) = {
        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        rt.block_on(async {
            let config = Config::load().await;
            let window = data::Window::load().await;

            (config, window)
        })
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

    let settings = settings(&config_load);
    let log_stream = Mutex::new(Some(log_stream));

    //tarkah: guess we need to move some stuff into the Halloy::new now.
    iced::daemon(
        move || {
            let log_stream = log_stream
                .lock()
                .unwrap()
                .take()
                .expect("will only panic if using iced_devtools");

            Halloy::new(
                config_load.clone(),
                window_load.clone(),
                destination.clone(),
                log_stream,
                // we start with an unspecified mode because we are guaranteed to
                // receive a message from mundy containing the correct mode on startup.
                appearance::Mode::Unspecified,
            )
        },
        Halloy::update,
        Halloy::view,
    )
    .title(Halloy::title)
    .theme(Halloy::theme)
    .scale_factor(Halloy::scale_factor)
    .subscription(Halloy::subscription)
    .settings(settings)
    .run()
    .inspect_err(|err| log::error!("{}", err))?;

    Ok(())
}

fn settings(config_load: &Result<Config, config::Error>) -> iced::Settings {
    let default_text_size = config_load
        .as_ref()
        .ok()
        .and_then(|config| config.font.size)
        .map_or(theme::TEXT_SIZE, f32::from);

    iced::Settings {
        default_font: font::MONO.clone().into(),
        default_text_size: default_text_size.into(),
        id: None,
        antialiasing: false,
        fonts: font::load(),
    }
}

fn handle_irc_error(e: anyhow::Error) {
    log::error!("{:#}", e);
}

struct Halloy {
    version: Version,
    screen: Screen,
    current_mode: appearance::Mode,
    theme: Theme,
    config: Config,
    clients: data::client::Map,
    servers: server::Map,
    modal: Option<Modal>,
    main_window: Window,
    pending_logs: Vec<data::log::Record>,
    notifications: Notifications,
}

impl Halloy {
    pub fn load_from_state(
        main_window: window::Id,
        config_load: Result<Config, config::Error>,
        current_mode: appearance::Mode,
    ) -> (Halloy, Task<Message>) {
        let main_window = Window::new(main_window);

        let load_dashboard = |config| match data::Dashboard::load() {
            Ok(dashboard) => {
                screen::Dashboard::restore(dashboard, config, &main_window)
            }
            Err(error) => {
                log::warn!("failed to load dashboard: {error}");

                screen::Dashboard::empty(config, &main_window)
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
                current_mode,
                theme: current_mode.theme(&config.appearance.selected).into(),
                clients: data::client::Map::default(),
                servers: config.servers.clone(),
                config,
                modal: None,
                main_window,
                pending_logs: vec![],
                notifications: Notifications::new(),
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
    OnConnect(Server, client::on_connect::Event),
}

impl Halloy {
    fn new(
        config_load: Result<Config, config::Error>,
        window_load: Result<data::Window, window::Error>,
        url_received: Option<data::Url>,
        log_stream: ReceiverStream<Vec<logger::Record>>,
        current_mode: appearance::Mode,
    ) -> (Halloy, Task<Message>) {
        let data::Window { size, position } = window_load.unwrap_or_default();
        let position =
            position.map(window::Position::Specific).unwrap_or_default();

        let (main_window, open_main_window) = window::open(window::Settings {
            size,
            position,
            min_size: Some(window::MIN_SIZE),
            exit_on_close_request: false,
            ..window::settings()
        });

        let (mut halloy, command) =
            Halloy::load_from_state(main_window, config_load, current_mode);
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
                        .preview_theme_in_editor(
                            colors,
                            &self.main_window,
                            &mut self.theme,
                        )
                        .map(Message::Dashboard);
                }
            }
            data::Url::Unknown(url) => {
                log::warn!("Received unknown url: {url}");
            }
        }

        Task::none()
    }

    fn title(&self, _window_id: window::Id) -> String {
        String::from("Halloy")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppearanceReloaded(appearance) => {
                self.config.appearance = appearance;
                Task::none()
            }
            Message::ScreenConfigReloaded(updated) => {
                let (halloy, command) = Halloy::load_from_state(
                    self.main_window.id,
                    updated,
                    self.current_mode,
                );
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
                let track = dashboard.track(&self.config);

                let event_task = match event {
                    Some(dashboard::Event::ConfigReloaded(config)) => {
                        match config {
                            Ok(updated) => {
                                let removed_servers = self
                                    .servers
                                    .keys()
                                    .filter(|server| {
                                        !updated.servers.contains(server)
                                    })
                                    .cloned()
                                    .collect::<Vec<_>>();

                                self.servers = updated.servers.clone();
                                self.theme = self
                                    .current_mode
                                    .theme(&updated.appearance.selected)
                                    .into();
                                self.config = updated;

                                for server in removed_servers {
                                    self.clients.quit(&server, None);
                                }
                            }
                            Err(error) => {
                                self.modal = Some(
                                    Modal::ReloadConfigurationError(error),
                                );
                            }
                        };
                        Task::none()
                    }
                    Some(dashboard::Event::ReloadThemes) => {
                        Task::future(Config::load())
                            .and_then(|config| Task::done(config.appearance))
                            .map(Message::AppearanceReloaded)
                    }
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
                    Some(dashboard::Event::OpenUrl(
                        url,
                        prompt_before_open,
                    )) => {
                        let Some((id, _, _)) = dashboard.get_focused() else {
                            return Task::none();
                        };

                        if prompt_before_open {
                            self.modal = Some(Modal::PromptBeforeOpenUrl {
                                url,
                                window: id,
                            });
                        } else {
                            let _ = open::that_detached(url);
                        }

                        Task::none()
                    }
                    Some(dashboard::Event::ImagePreview(path, url)) => {
                        let Some((id, _, _)) = dashboard.get_focused() else {
                            return Task::none();
                        };

                        self.modal = Some(Modal::ImagePreview {
                            source: path,
                            url,
                            timer: None,
                            window: id,
                        });
                        Task::none()
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
                    Some(help::Event::RefreshConfiguration) => Task::perform(
                        Config::load(),
                        Message::ScreenConfigReloaded,
                    ),
                    None => Task::none(),
                }
            }
            Message::Welcome(message) => {
                let Screen::Welcome(welcome) = &mut self.screen else {
                    return Task::none();
                };

                match welcome.update(message) {
                    Some(welcome::Event::RefreshConfiguration) => {
                        Task::perform(
                            Config::load(),
                            Message::ScreenConfigReloaded,
                        )
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
                        Task::perform(
                            Config::load(),
                            Message::ScreenConfigReloaded,
                        )
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
                        // Initial is sent when first trying to connect
                        dashboard
                            .broadcast(
                                &server,
                                &self.config,
                                sent_time,
                                Broadcast::Connecting,
                            )
                            .map(Message::Dashboard)
                    } else {
                        self.notifications.notify(
                            &self.config.notifications,
                            &Notification::Disconnected,
                            &server,
                        );

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

                    let broadcast = if is_initial {
                        self.notifications.notify(
                            &self.config.notifications,
                            &Notification::Connected,
                            &server,
                        );

                        dashboard
                            .broadcast(
                                &server,
                                &self.config,
                                sent_time,
                                Broadcast::Connected,
                            )
                            .map(Message::Dashboard)
                    } else {
                        self.notifications.notify(
                            &self.config.notifications,
                            &Notification::Reconnected,
                            &server,
                        );

                        dashboard
                            .broadcast(
                                &server,
                                &self.config,
                                sent_time,
                                Broadcast::Reconnected,
                            )
                            .map(Message::Dashboard)
                    };

                    let refocus_pane =
                        dashboard.refocus_pane().map(Message::Dashboard);

                    Task::batch(vec![broadcast, refocus_pane])
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
                            let events = match self.clients.receive(
                                &server,
                                message,
                                &self.config.ctcp,
                            ) {
                                Ok(events) => events,
                                Err(e) => {
                                    handle_irc_error(e);
                                    return vec![];
                                }
                            };

                            let mut commands = vec![];

                            for event in events {
                                // Resolve a user using client state which stores attributes
                                let resolve_user_attributes =
                                    |user: &User, channel: &target::Channel| {
                                        self.clients
                                            .resolve_user_attributes(&server, channel, user)
                                            .cloned()
                                    };

                                let channel_users = |channel: &target::Channel| -> &[User] {
                                    self.clients.get_channel_users(&server, channel)
                                };

                                let chantypes = self.clients.get_chantypes(&server);
                                let statusmsg = self.clients.get_statusmsg(&server);
                                let casemapping = self.clients.get_casemapping(&server);

                                match event {
                                    data::client::Event::Single(encoded, our_nick) => {
                                        if let Some(message) = message::decode(
                                            encoded,
                                            our_nick,
                                            &self.config,
                                            resolve_user_attributes,
                                            channel_users,
                                            chantypes,
                                            statusmsg,
                                            casemapping,
                                        ) {
                                            commands.push(
                                                dashboard
                                                    .record_decoded(
                                                        &server,
                                                        message,
                                                    )
                                                    .map(Message::Dashboard),
                                            );
                                        }
                                    }
                                    data::client::Event::PrivOrNotice(
                                        encoded,
                                        our_nick,
                                        highlight_notification_enabled,
                                    ) => {
                                        if let Some(message) = message::decode(
                                            encoded,
                                            our_nick,
                                            &self.config,
                                            resolve_user_attributes,
                                            channel_users,
                                            chantypes,
                                            statusmsg,
                                            casemapping,
                                        ) {
                                            if let Some((message, channel, user)) =
                                                message.into_highlight(&server)
                                            {
                                                let message_text = message.text();

                                                commands.push(
                                                    dashboard
                                                        .record_highlight(message)
                                                        .map(Message::Dashboard),
                                                );

                                                if highlight_notification_enabled {
                                                    self.notifications.notify(
                                                        &self.config.notifications,
                                                        &Notification::Highlight {
                                                            user,
                                                            channel,
                                                            message: message_text,
                                                        },
                                                        &server,
                                                    );
                                                }
                                            }

                                            commands.push(
                                                dashboard
                                                    .record_decoded(
                                                        &server,
                                                        message,
                                                    )
                                                    .map(Message::Dashboard),
                                            );
                                        }
                                    }
                                    data::client::Event::WithTarget(encoded, our_nick, target) => {
                                        if let Some(message) = message::decode(
                                            encoded,
                                            our_nick,
                                            &self.config,
                                            resolve_user_attributes,
                                            channel_users,
                                            chantypes,
                                            statusmsg,
                                            casemapping,
                                        ) {
                                            commands.push(
                                                dashboard
                                                    .record_decoded(
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
                                            logged_in,
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
                                                            logged_in,
                                                            user_channels: channels,
                                                        },
                                                    )
                                                    .map(Message::Dashboard),
                                            );
                                        }
                                    },
                                    data::client::Event::FileTransferRequest(request) => {
                                        if let Some(command) = dashboard.receive_file_transfer(
                                            &server,
                                            chantypes,
                                            statusmsg,
                                            casemapping,
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
                                                Target::Channel(channel),
                                                server_time,
                                            )
                                            .map(Message::Dashboard);

                                        commands.push(command);
                                    }
                                    data::client::Event::LoggedIn(server_time) => {
                                        if self.clients.get_server_supports_chathistory(&server) {
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
                                    }
                                    data::client::Event::ChatHistoryTargetReceived(
                                        target,
                                        server_time,
                                    ) => {
                                        let command = dashboard
                                            .load_metadata(
                                                &self.clients,
                                                server.clone(),
                                                target,
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
                                    data::client::Event::DirectMessage(encoded, our_nick, user) => {
                                        if let Some(decoded) = message::decode(
                                            encoded,
                                            our_nick,
                                            &self.config,
                                            resolve_user_attributes,
                                            channel_users,
                                            chantypes,
                                            statusmsg,
                                            casemapping,
                                        ) {
                                            if let (Ok(query), Decoded::Message(message)) = (target::Query::parse(
                                                user.as_str(),
                                                chantypes,
                                                statusmsg,
                                                casemapping,
                                            ), decoded) {
                                                if dashboard.history().has_unread(
                                                    &history::Kind::Query(server.clone(), query),
                                                ) || !self.main_window.focused
                                                {
                                                    self.notifications.notify(
                                                        &self.config.notifications,
                                                        &Notification::DirectMessage{
                                                            user,
                                                            message: message.text(),
                                                        },
                                                        &server,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    data::client::Event::MonitoredOnline(users) => {
                                        self.notifications.notify(
                                            &self.config.notifications,
                                            &Notification::MonitoredOnline(users),
                                            &server,
                                        );
                                    }
                                    data::client::Event::MonitoredOffline(users) => {
                                        self.notifications.notify(
                                            &self.config.notifications,
                                            &Notification::MonitoredOffline(users),
                                            &server,
                                        );
                                    }
                                    data::client::Event::OnConnect(
                                        on_connect,
                                    ) => {
                                        let server = server.clone();
                                        commands.push(
                                            Task::stream(on_connect)
                                                .map(move |event| {
                                                    Message::OnConnect(
                                                        server.clone(),
                                                        event
                                                    )
                                                })
                                        );
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
                    match &mut self.screen {
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
                    }
                }
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
                        )
                        .map(Message::Dashboard);
                }

                Task::none()
            }
            Message::Tick(now) => {
                if let Err(e) = self.clients.tick(now) {
                    handle_irc_error(e);
                }

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

                let (command, event) = modal.update(message);

                if let Some(event) = event {
                    match event {
                        modal::Event::CloseModal => {
                            self.modal = None;
                        }
                        modal::Event::AcceptNewServer => {
                            if let Some(Modal::ServerConnect {
                                server,
                                config,
                                ..
                            }) = self.modal.take()
                            {
                                let existing_entry =
                                    self.servers.entries().find(|entry| {
                                        entry.server == server
                                            || entry.config.server
                                                == config.server
                                    });

                                // If server already exists, we only want to join the new channels
                                if let Some(entry) = existing_entry {
                                    let chantypes =
                                        self.clients.get_chantypes(&server);
                                    let statusmsg =
                                        self.clients.get_statusmsg(&server);
                                    let casemapping =
                                        self.clients.get_casemapping(&server);

                                    self.clients.join(
                                        &entry.server,
                                        &config
                                            .channels
                                            .iter()
                                            .filter_map(|channel| {
                                                target::Channel::parse(
                                                    channel,
                                                    chantypes,
                                                    statusmsg,
                                                    casemapping,
                                                )
                                                .ok()
                                            })
                                            .collect::<Vec<_>>(),
                                    );
                                } else {
                                    self.servers.insert(server, config);
                                }
                            }
                        }
                    }
                }

                command.map(Message::Modal)
            }
            Message::RouteReceived(route) => {
                log::info!("RouteReceived: {:?}", route);

                if let Ok(url) = route.parse() {
                    return self.handle_url(url);
                };

                Task::none()
            }
            Message::Window(id, event) => {
                if id == self.main_window.id {
                    match event {
                        window::Event::Moved(position) => {
                            self.main_window.position = Some(position);
                        }
                        window::Event::Resized(size) => {
                            self.main_window.size = size;
                        }
                        window::Event::Focused => {
                            self.main_window.focused = true;
                        }
                        window::Event::Unfocused => {
                            self.main_window.focused = false;
                        }
                        window::Event::Opened { position, size } => {
                            self.main_window.opened(position, size);
                        }
                        window::Event::CloseRequested => {
                            if let Screen::Dashboard(dashboard) =
                                &mut self.screen
                            {
                                return dashboard
                                    .exit(&self.config)
                                    .map(Message::Dashboard);
                            } else {
                                return iced::exit();
                            }
                        }
                    }

                    let mut tasks = vec![Task::perform(
                        data::Window::from(self.main_window).save(),
                        Message::WindowSettingsSaved,
                    )];

                    if let Some(Screen::Dashboard(dashboard)) =
                        matches!(event, window::Event::Focused)
                            .then_some(&mut self.screen)
                    {
                        tasks.push(
                            dashboard
                                .focus_window_pane(self.main_window.id)
                                .map(Message::Dashboard),
                        );
                    }

                    Task::batch(tasks)
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
                    log::error!("window settings failed to save: {:?}", err);
                }

                Task::none()
            }
            Message::AppearanceChange(mode) => {
                if let data::appearance::Selected::Dynamic { .. } =
                    &self.config.appearance.selected
                {
                    self.current_mode = mode;
                    self.theme = self
                        .current_mode
                        .theme(&self.config.appearance.selected)
                        .into();
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
            Message::OnConnect(server, event) => match event {
                client::on_connect::Event::OpenBuffers(targets) => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Task::none();
                    };

                    let mut commands = vec![];

                    for target in targets {
                        let buffer_action = match target {
                            Target::Channel(_) => {
                                self.config.actions.buffer.message_channel
                            }
                            Target::Query(_) => {
                                self.config.actions.buffer.message_user
                            }
                        };

                        commands.push(dashboard.open_target(
                            server.clone(),
                            target,
                            &mut self.clients,
                            buffer_action,
                            &self.config,
                        ));
                    }

                    Task::batch(commands).map(Message::Dashboard)
                }
            },
        }
    }

    fn view(&self, id: window::Id) -> Element<Message> {
        // The height margin varies across different operating systems due to design differences.
        // For instance, on macOS, the menubar is hidden, resulting in a need for additional padding to accommodate the
        // space occupied by the traffic light buttons.
        let height_margin = if cfg!(target_os = "macos") { 20 } else { 0 };

        // Main window.
        if id == self.main_window.id {
            let screen = match &self.screen {
                Screen::Dashboard(dashboard) => dashboard
                    .view(
                        &self.clients,
                        &self.version,
                        &self.config,
                        &self.theme,
                    )
                    .map(Message::Dashboard),
                Screen::Help(help) => help.view().map(Message::Help),
                Screen::Welcome(welcome) => {
                    welcome.view().map(Message::Welcome)
                }
                Screen::Migration(migration) => {
                    migration.view().map(Message::Migration)
                }
                Screen::Exit { .. } => column![].into(),
            };

            let content = container(
                container(screen)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::container::general),
            )
            .padding(padding::top(height_margin));

            // Modals might have a id representing which window to be presented on.
            // If modal has no id, we show them on main_window.
            match (&self.modal, &self.screen) {
                (Some(modal), Screen::Dashboard(_))
                    if modal.window_id() == Some(self.main_window.id)
                        || modal.window_id().is_none() =>
                {
                    widget::modal(
                        content,
                        modal.view().map(Message::Modal),
                        || Message::Modal(modal::Message::Cancel),
                    )
                }
                _ => column![content].into(),
            }
        // Popped out window.
        } else if let Screen::Dashboard(dashboard) = &self.screen {
            let content = container(
                dashboard
                    .view_window(id, &self.clients, &self.config, &self.theme)
                    .map(Message::Dashboard),
            )
            .padding(padding::top(height_margin));

            // Modals might have a id representing which window to be presented on.
            // If modal id match the current id we show it.
            match &self.modal {
                Some(modal) if modal.window_id() == Some(id) => widget::modal(
                    content,
                    modal.view().map(Message::Modal),
                    || Message::Modal(modal::Message::Cancel),
                ),
                _ => column![content].into(),
            }
        } else {
            column![].into()
        }
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

        let mut subscriptions = vec![
            url::listen().map(Message::RouteReceived),
            events().map(|(window, event)| Message::Event(window, event)),
            window::events()
                .map(|(window, event)| Message::Window(window, event)),
            tick,
            streams,
        ];

        // We only want to listen for appearance changes if user has dynamic themes.
        if self.config.appearance.selected.is_dynamic() {
            subscriptions.push(
                appearance::subscription().map(Message::AppearanceChange),
            );
        }

        Subscription::batch(subscriptions)
    }
}
