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
mod open_url;
mod platform_specific;
mod screen;
mod stream;
mod unix_signal;
mod url;
mod widget;
mod window;

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{env, mem};

use appearance::{Theme, theme};
use data::config::{self, Config};
use data::history::filter::FilterChain;
use data::message::{self, Broadcast};
use data::reaction::Reaction;
use data::target::{self, Target};
use data::version::Version;
use data::{
    Notification, Server, Url, User, client, environment, history, server,
    version,
};
use iced::widget::{column, container};
use iced::{Length, Subscription, Task, padding};
use screen::{dashboard, help, welcome};
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

    let logs_config = Config::load_logs().unwrap_or_default();

    // spin up a single-threaded tokio runtime to run the logs deletion and
    // config loading tasks to completion we don't want to wrap our whole
    // program with a runtime since iced starts its own.
    let rt = runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let _ = rt.block_on(async {
        tokio::join!(
            history::delete(&history::Kind::Logs),
            history::metadata::delete(&history::Kind::Logs)
        )
    });

    let log_stream =
        logger::setup(is_debug, logs_config).expect("setup logging");
    log::info!("halloy {} has started", environment::formatted_version());
    log::info!("config dir: {:?}", environment::config_dir());
    log::info!("data dir: {:?}", environment::data_dir());

    let (config_load, window_load) = {
        rt.block_on(async {
            let config = Config::load().await;
            let window = data::Window::load().await;

            (config, window)
        })
    };

    // Futures have only been run via block_on, so we should be able to
    // shutdown_background without leaks
    rt.shutdown_background();

    // DANGER ZONE - font must be set using config
    // before we do any iced related stuff w/ it
    font::set(config_load.as_ref().ok());

    let destination = data::Url::find_in(std::env::args());
    if let Some(loc) = &destination
        && ipc::connect_and_send(loc.to_string())
    {
        return Ok(());
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
    .inspect_err(|err| log::error!("{err}"))?;

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
        vsync: true,
    }
}

fn handle_irc_error(e: anyhow::Error) {
    log::error!("{e:#}");
}

struct Halloy {
    version: Version,
    screen: Screen,
    current_mode: appearance::Mode,
    theme: Theme,
    config: Config,
    clients: data::client::Map,
    servers: server::Map,
    controllers: stream::Map,
    modal: Option<Modal>,
    main_window: Window,
    focused_window: Option<window::Id>,
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
        let load_dashboard = |config: &Config| match data::Dashboard::load() {
            Ok(dashboard) => {
                if config.pane.restore_on_launch {
                    screen::Dashboard::restore(dashboard, config, &main_window)
                } else {
                    screen::Dashboard::empty(&main_window, config)
                }
            }
            Err(error) => {
                if data::Dashboard::exists().is_ok_and(|exists| exists) {
                    log::warn!("failed to load dashboard: {error}");
                } else {
                    // Most likely this means it is the user's first launch,
                    // downgrade severity to info
                    log::info!("failed to load dashboard: {error}");
                }

                screen::Dashboard::empty(&main_window, config)
            }
        };

        let (screen, servers, config, command) = match config_load {
            Ok(config) => {
                let mut servers: server::Map = config.servers.clone().into();
                servers.set_order(config.sidebar.order_by);
                let (mut screen, command) = load_dashboard(&config);
                screen.init_filters(&servers, &data::client::Map::default());
                (
                    Screen::Dashboard(screen),
                    servers,
                    config,
                    command.map(Message::Dashboard),
                )
            }
            // Show regular welcome screen for new users.
            Err(config::Error::ConfigMissing) => (
                Screen::Welcome(screen::Welcome::default()),
                server::Map::default(),
                Config::default(),
                Task::none(),
            ),
            Err(error) => (
                Screen::Help(screen::Help::new(error)),
                server::Map::default(),
                Config::default(),
                Task::none(),
            ),
        };

        let notifications = Notifications::new(&config);

        (
            Halloy {
                version: Version::new(),
                screen,
                current_mode,
                theme: current_mode.theme(&config.appearance.selected).into(),
                clients: data::client::Map::default(),
                servers,
                controllers: stream::Map::default(),
                config,
                modal: None,
                main_window,
                focused_window: None,
                pending_logs: vec![],
                notifications,
            },
            command,
        )
    }
}

pub enum Screen {
    Dashboard(screen::Dashboard),
    Help(screen::Help),
    Welcome(screen::Welcome),
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
    Event(window::Id, Event),
    Tick(Instant),
    Version(Option<String>),
    Modal(modal::Message),
    RouteReceived(String),
    AppearanceChange(appearance::Mode),
    Window(window::Id, window::Event),
    WindowSettingsSaved(Result<(), window::Error>),
    WindowMaximizeChecked(bool),
    Logging(Vec<logger::Record>),
    OnConnect(Server, client::on_connect::Event),
    UnixSignal(i32),
    ConfigReloaded(Result<Config, config::Error>),
}

impl Halloy {
    fn new(
        config_load: Result<Config, config::Error>,
        window_load: Result<data::Window, window::Error>,
        url_received: Option<data::Url>,
        log_stream: ReceiverStream<Vec<logger::Record>>,
        current_mode: appearance::Mode,
    ) -> (Halloy, Task<Message>) {
        let data::Window {
            size,
            position,
            fullscreen,
            maximized,
        } = window_load.unwrap_or_default();

        let default_config = Config::default();
        let config = config_load.as_ref().unwrap_or(&default_config);
        let proxy_config = config.proxy.clone();
        let check_for_update_on_launch = config.check_for_update_on_launch;

        let (main_window, open_main_window) = window::open(window::Settings {
            size: fullscreen.unwrap_or(size),
            position: position
                .map(window::Position::Specific)
                .unwrap_or_default(),
            min_size: Some(window::MIN_SIZE),
            exit_on_close_request: false,
            fullscreen: fullscreen.is_some(),
            ..window::settings(config)
        });

        let (mut halloy, command) =
            Halloy::load_from_state(main_window, config_load, current_mode);

        halloy.main_window.fullscreen = fullscreen;
        halloy.main_window.maximized = maximized;
        halloy.main_window.windowed_position = position;
        halloy.main_window.windowed_size = size;

        let open_task = if maximized {
            open_main_window.then(move |_| window::maximize(main_window, true))
        } else {
            open_main_window.then(|_| Task::none())
        };

        let mut commands = vec![
            open_task,
            command,
            Task::stream(log_stream).map(Message::Logging),
        ];

        if check_for_update_on_launch {
            commands.push(Task::perform(
                version::latest_remote_version(proxy_config),
                Message::Version,
            ));
        }

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
                    server: server.into(),
                    config,
                });
            }
            data::Url::Theme { styles, .. } => {
                if let Screen::Dashboard(dashboard) = &mut self.screen {
                    return dashboard
                        .preview_theme_in_editor(
                            styles,
                            &self.main_window,
                            &mut self.theme,
                            &self.config,
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
            Message::ConfigReloaded(config) => {
                self.config_file_reloaded(config)
            }
            Message::AppearanceReloaded(appearance) => {
                self.config.appearance = appearance;
                Task::none()
            }
            Message::ScreenConfigReloaded(updated) => {
                let saved_window = self.main_window;
                let (mut halloy, command) = Halloy::load_from_state(
                    self.main_window.id,
                    updated,
                    self.current_mode,
                );
                halloy.main_window = saved_window;
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
                    &mut self.controllers,
                    &self.servers,
                    &mut self.theme,
                    &self.version,
                    &self.config,
                    &self.main_window,
                );

                // Retrack after dashboard state changes
                let track = dashboard.track(Some(&self.clients));

                let event_task = match event {
                    Some(dashboard::Event::ToggleFullscreen) => {
                        self.main_window.toggle_fullscreen();
                        Task::perform(
                            data::Window::from(self.main_window).save(),
                            Message::WindowSettingsSaved,
                        )
                    }
                    Some(dashboard::Event::ConfigReloaded(config)) => {
                        self.config_file_reloaded(config)
                    }
                    Some(dashboard::Event::ReloadThemes) => {
                        Task::future(Config::load()).then(|config| match config
                        {
                            Ok(config) => Task::done(
                                Message::AppearanceReloaded(config.appearance),
                            ),
                            Err(_) => Task::none(),
                        })
                    }
                    Some(dashboard::Event::QuitServer(server, reason)) => {
                        for bouncer_network in
                            self.servers.get_bouncer_networks(&server)
                        {
                            self.clients.quit(bouncer_network, reason.clone());
                        }

                        self.clients.quit(&server, reason);

                        Task::none()
                    }
                    Some(dashboard::Event::IrcError(e)) => {
                        handle_irc_error(e);
                        Task::none()
                    }
                    Some(dashboard::Event::Exit) => {
                        let pending_exit = self.controllers.exit(
                            &self.config.buffer.commands.quit.default_reason,
                        );

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
                            let _ = open_url::open(url);
                        }

                        Task::none()
                    }
                    Some(dashboard::Event::OpenAbout {
                        version,
                        commit,
                        system_information,
                    }) => {
                        self.modal =
                            Some(Modal::About(modal::about::About::new(
                                version,
                                commit,
                                system_information,
                            )));

                        Task::none()
                    }
                    Some(dashboard::Event::OpenServer(server)) => {
                        if let Ok(url) = Url::from_str(&server)
                            && matches!(url, Url::ServerConnect { .. })
                        {
                            self.handle_url(url)
                        } else {
                            Task::none()
                        }
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
                    Some(dashboard::Event::Remove(server)) => {
                        self.remove(server)
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
                        Task::none()
                    } else {
                        let request_attention = if !self.main_window.focused {
                            self.notifications.notify(
                                &self.config.notifications,
                                &Notification::Disconnected,
                                &server,
                                dashboard
                                    .find_window_with_server(&server)
                                    .unwrap_or(self.main_window.id),
                            )
                        } else {
                            None
                        };

                        let mut tasks = vec![
                            dashboard
                                .broadcast(
                                    &server,
                                    self.clients.get_casemapping(&server),
                                    &self.config,
                                    sent_time,
                                    Broadcast::Disconnected { error },
                                )
                                .map(Message::Dashboard),
                        ];

                        if let Some(request_attention) = request_attention {
                            tasks.push(request_attention);
                        }

                        Task::batch(tasks)
                    }
                }
                stream::Update::Connecting { server, sent_time } => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Task::none();
                    };

                    // Initial is sent when first trying to connect
                    dashboard
                        .broadcast(
                            &server,
                            self.clients.get_casemapping(&server),
                            &self.config,
                            sent_time,
                            Broadcast::Connecting,
                        )
                        .map(Message::Dashboard)
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

                    let (notification, broadcast_kind) = if is_initial {
                        (Notification::Connected, Broadcast::Connected)
                    } else {
                        (Notification::Reconnected, Broadcast::Reconnected)
                    };

                    let request_attention = if !self.main_window.focused {
                        self.notifications.notify(
                            &self.config.notifications,
                            &notification,
                            &server,
                            dashboard
                                .find_window_with_server(&server)
                                .unwrap_or(self.main_window.id),
                        )
                    } else {
                        None
                    };

                    let broadcast = dashboard
                        .broadcast(
                            &server,
                            self.clients.get_casemapping(&server),
                            &self.config,
                            sent_time,
                            broadcast_kind,
                        )
                        .map(Message::Dashboard);

                    let refocus_pane =
                        dashboard.refocus_pane().map(Message::Dashboard);

                    let mut tasks = vec![broadcast, refocus_pane];

                    if let Some(request_attention) = request_attention {
                        tasks.push(request_attention);
                    }

                    Task::batch(tasks)
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
                            self.clients.get_casemapping(&server),
                            &self.config,
                            sent_time,
                            Broadcast::ConnectionFailed { error },
                        )
                        .map(Message::Dashboard)
                }
                stream::Update::MessagesReceived(server, messages) => {
                    self.handle_messages_received(server, messages)
                }
                stream::Update::Remove(server) => self.remove(server),
                stream::Update::Controller { server, controller } => {
                    self.controllers.insert(server, controller);

                    Task::none()
                }
                stream::Update::UpdateConfiguration {
                    server,
                    updated_config,
                } => {
                    let events = self.clients.update_config(
                        &server,
                        updated_config.clone(),
                        false,
                    );

                    let mut bouncer_network_events = vec![];

                    for bouncer_network in
                        self.servers.get_bouncer_networks(&server)
                    {
                        bouncer_network_events.push((
                            bouncer_network.clone(),
                            self.clients.update_config(
                                bouncer_network,
                                updated_config.bouncer_config().into(),
                                false,
                            ),
                        ));
                    }

                    if let Screen::Dashboard(dashboard) = &mut self.screen {
                        let commands = handle_client_events(
                            &server,
                            events,
                            dashboard,
                            &mut self.clients,
                            &self.config,
                            &mut self.notifications,
                            &mut self.servers,
                            &mut self.controllers,
                            &self.main_window,
                            self.focused_window,
                        );

                        if bouncer_network_events.is_empty() {
                            return commands;
                        }

                        let mut bouncer_network_commands = vec![];

                        for (bouncer_network, events) in bouncer_network_events
                        {
                            bouncer_network_commands.push(
                                handle_client_events(
                                    &bouncer_network,
                                    events,
                                    dashboard,
                                    &mut self.clients,
                                    &self.config,
                                    &mut self.notifications,
                                    &mut self.servers,
                                    &mut self.controllers,
                                    &self.main_window,
                                    self.focused_window,
                                ),
                            );
                        }

                        return commands
                            .chain(Task::batch(bouncer_network_commands));
                    }

                    Task::none()
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
                    dashboard.tick(now, &self.clients).map(Message::Dashboard)
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
                                    let events = self.clients.update_config(
                                        &entry.server,
                                        Arc::new(config),
                                        true,
                                    );

                                    if let Screen::Dashboard(dashboard) =
                                        &mut self.screen
                                    {
                                        let commands = handle_client_events(
                                            &server,
                                            events,
                                            dashboard,
                                            &mut self.clients,
                                            &self.config,
                                            &mut self.notifications,
                                            &mut self.servers,
                                            &mut self.controllers,
                                            &self.main_window,
                                            self.focused_window,
                                        );

                                        return command
                                            .map(Message::Modal)
                                            .chain(commands);
                                    }
                                } else {
                                    self.servers
                                        .insert(server, Arc::new(config));
                                }
                            }
                        }
                    }
                }

                command.map(Message::Modal)
            }
            Message::RouteReceived(route) => {
                log::info!("RouteReceived: {route:?}");

                if let Ok(url) = route.parse() {
                    return self.handle_url(url);
                };

                Task::none()
            }
            Message::Window(id, event) => {
                match &event {
                    window::Event::Focused => {
                        self.focused_window = Some(id);
                    }
                    window::Event::Unfocused => {
                        if self.focused_window == Some(id) {
                            self.focused_window = None;
                        }
                    }
                    window::Event::Opened { .. }
                    | window::Event::Moved(_)
                    | window::Event::Resized(_)
                    | window::Event::CloseRequested => {}
                }

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
                            let save = Task::perform(
                                data::Window::from(self.main_window).save(),
                                Message::WindowSettingsSaved,
                            );

                            if let Screen::Dashboard(dashboard) =
                                &mut self.screen
                            {
                                return save.chain(
                                    dashboard
                                        .exit(&mut self.clients, &self.config)
                                        .map(Message::Dashboard),
                                );
                            } else {
                                return save.chain(iced::exit());
                            }
                        }
                    }

                    let mut tasks = vec![
                        iced::window::is_maximized(self.main_window.id)
                            .map(Message::WindowMaximizeChecked),
                    ];

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
            Message::WindowMaximizeChecked(is_maximized) => {
                self.main_window.update_maximize(is_maximized);
                Task::perform(
                    data::Window::from(self.main_window).save(),
                    Message::WindowSettingsSaved,
                )
            }
            Message::WindowSettingsSaved(result) => {
                if let Err(err) = result {
                    log::error!("window settings failed to save: {err:?}");
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
                        .filter(|record| {
                            record.level <= self.config.logs.pane_level
                        })
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
                client::on_connect::Event::LeaveBuffers(targets, reason) => {
                    let Screen::Dashboard(dashboard) = &mut self.screen else {
                        return Task::none();
                    };

                    let mut commands = vec![];

                    for target in targets {
                        commands.push(dashboard.leave_server_target(
                            &mut self.clients,
                            &self.config,
                            server.clone(),
                            target,
                            reason.clone(),
                        ));
                    }

                    Task::batch(commands).map(Message::Dashboard)
                }
            },
            Message::UnixSignal(signal) => match signal {
                #[cfg(target_family = "unix")]
                signal_hook::consts::SIGUSR1 => {
                    Task::perform(Config::load(), Message::ConfigReloaded)
                }
                _ => Task::none(),
            },
        }
    }

    fn view(&self, id: window::Id) -> Element<'_, Message> {
        let platform_specific_padding =
            platform_specific::content_padding(&self.config);

        // Main window.
        if id == self.main_window.id {
            let screen = match &self.screen {
                Screen::Dashboard(dashboard) => dashboard
                    .view(
                        &self.servers,
                        &self.clients,
                        &self.version,
                        &self.config,
                        &self.theme,
                    )
                    .map(Message::Dashboard),
                Screen::Help(help) => help.view(&self.theme).map(Message::Help),
                Screen::Welcome(welcome) => {
                    welcome.view(&self.theme).map(Message::Welcome)
                }
                Screen::Exit { .. } => column![].into(),
            };

            let content = container(
                container(screen)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::container::root),
            )
            .padding(padding::top(platform_specific_padding));

            // Modals might have a id representing which window to be presented on.
            // If modal has no id, we show them on main_window.
            match (&self.modal, &self.screen) {
                (Some(modal), Screen::Dashboard(_))
                    if modal.window_id() == Some(self.main_window.id)
                        || modal.window_id().is_none() =>
                {
                    widget::modal(
                        content,
                        modal.view(&self.theme).map(Message::Modal),
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
            .padding(padding::top(platform_specific_padding));

            // Modals might have a id representing which window to be presented on.
            // If modal id match the current id we show it.
            match &self.modal {
                Some(modal) if modal.window_id() == Some(id) => widget::modal(
                    content,
                    modal.view(&self.theme).map(Message::Modal),
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

    fn scale_factor(&self, _window: window::Id) -> f32 {
        f32::from(self.config.scale_factor)
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

        if cfg!(target_family = "unix") {
            subscriptions
                .push(unix_signal::subscription().map(Message::UnixSignal));
        }

        // We only want to listen for appearance changes if user has dynamic themes.
        if self.config.appearance.selected.is_dynamic() {
            subscriptions.push(
                appearance::subscription().map(Message::AppearanceChange),
            );
        }

        Subscription::batch(subscriptions)
    }

    fn config_file_reloaded(
        &mut self,
        config: Result<Config, config::Error>,
    ) -> Task<Message> {
        match config {
            Ok(updated) => {
                let removed_servers = self
                    .servers
                    .extract_if(|server, _| {
                        !updated.servers.contains(&server.name)
                    })
                    .collect::<Vec<_>>();

                for (server, config) in updated.servers.iter() {
                    let server = server.clone().into();

                    if let Some(existing) = self.servers.get_mut(&server) {
                        *existing = config.clone();

                        let bouncer_networks = self
                            .servers
                            .get_bouncer_networks(&server)
                            .cloned()
                            .collect::<Vec<_>>();

                        for bouncer_network in bouncer_networks {
                            if let Some(bouncer_network) =
                                self.servers.get_mut(&bouncer_network)
                            {
                                *bouncer_network =
                                    config.bouncer_config().into();
                            }
                        }

                        self.controllers.update_config(
                            &server,
                            config.clone(),
                            updated.proxy.clone(),
                        );
                    } else {
                        self.servers.insert(server, config.clone());
                    }
                }

                self.servers.set_order(updated.sidebar.order_by);

                self.theme = self
                    .current_mode
                    .theme(&updated.appearance.selected)
                    .into();

                // Load new notification sounds.
                self.notifications = Notifications::new(&updated);

                self.config = updated;

                for (server, _) in removed_servers {
                    self.clients.quit(
                        &server,
                        self.config.buffer.commands.quit.default_reason.clone(),
                    );
                }

                if let Screen::Dashboard(dashboard) = &mut self.screen {
                    dashboard.update_filters(
                        &self.servers,
                        &self.clients,
                        &self.config.buffer,
                    );

                    return dashboard
                        .reload_visible_previews(&self.clients, &self.config)
                        .map(Message::Dashboard);
                }
            }
            Err(error) => {
                self.modal = Some(Modal::ReloadConfigurationError(error));
            }
        }

        Task::none()
    }

    fn handle_messages_received(
        &mut self,
        server: Server,
        messages: Vec<message::Encoded>,
    ) -> Task<Message> {
        let mut all_events = vec![];
        for message in messages {
            match self.clients.receive(&server, message, &self.config) {
                Ok(events) => all_events.extend(events),
                Err(e) => handle_irc_error(e),
            }
        }

        let Screen::Dashboard(dashboard) = &mut self.screen else {
            return Task::none();
        };

        handle_client_events(
            &server,
            all_events,
            dashboard,
            &mut self.clients,
            &self.config,
            &mut self.notifications,
            &mut self.servers,
            &mut self.controllers,
            &self.main_window,
            self.focused_window,
        )
    }

    fn remove(&mut self, server: Server) -> Task<Message> {
        match &mut self.screen {
            Screen::Dashboard(_) => {
                self.controllers.end(
                    &server,
                    &self.config.buffer.commands.quit.default_reason,
                );

                self.servers.remove(&server);

                self.clients.remove(&server);

                Task::none()
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
}

fn handle_client_events(
    server: &Server,
    events: Vec<data::client::Event>,
    dashboard: &mut screen::Dashboard,
    clients: &mut data::client::Map,
    config: &Config,
    notifications: &mut Notifications,
    servers: &mut server::Map,
    controllers: &mut stream::Map,
    main_window: &Window,
    focused_window: Option<window::Id>,
) -> Task<Message> {
    use data::client::Event;

    let casemapping = clients.get_casemapping(server);

    let mut commands = vec![];
    let mut reactions = vec![];

    for event in events {
        match event {
            Event::Single(encoded, our_nick) => {
                handle_single_event(
                    server,
                    encoded,
                    our_nick,
                    dashboard,
                    &mut commands,
                    clients,
                    config,
                );
            }
            Event::PrivOrNotice(encoded, our_nick, notification_enabled) => {
                handle_priv_or_notice(
                    server,
                    encoded,
                    our_nick,
                    notification_enabled,
                    dashboard,
                    &mut commands,
                    clients,
                    config,
                    notifications,
                    main_window,
                    focused_window,
                );
            }
            Event::WithTarget(encoded, our_nick, target) => {
                handle_with_target_event(
                    server,
                    encoded,
                    our_nick,
                    target,
                    dashboard,
                    &mut commands,
                    clients,
                    config,
                );
            }
            Event::Broadcast(broadcast) => {
                handle_broadcast(
                    server,
                    broadcast,
                    dashboard,
                    &mut commands,
                    clients,
                    config,
                );
            }
            Event::FileTransferRequest(request) => {
                if let Some(command) = dashboard.receive_file_transfer(
                    server,
                    casemapping,
                    request,
                    config,
                ) {
                    commands.push(command.map(Message::Dashboard));
                }
            }
            Event::UpdateReadMarker(target, read_marker) => {
                commands.push(
                    dashboard
                        .update_read_marker(
                            history::Kind::from_target(server.clone(), target),
                            read_marker,
                        )
                        .map(Message::Dashboard),
                );
            }
            Event::JoinedChannel(channel, server_time) => {
                commands.push(
                    dashboard
                        .load_metadata(
                            clients,
                            server.clone(),
                            Target::Channel(channel),
                            server_time,
                        )
                        .map(Message::Dashboard),
                );
            }
            Event::LoggedIn(server_time) => {
                if clients.get_server_supports_chathistory(server)
                    && let Some(command) = dashboard
                        .load_chathistory_targets_timestamp(
                            clients,
                            server,
                            server_time,
                        )
                        .map(|cmd| cmd.map(Message::Dashboard))
                {
                    commands.push(command);
                }
            }
            Event::ChatHistoryTargetReceived(target, server_time) => {
                commands.push(
                    dashboard
                        .load_metadata(
                            clients,
                            server.clone(),
                            target,
                            server_time,
                        )
                        .map(Message::Dashboard),
                );
            }
            Event::ChatHistoryTargetsReceived(server_time) => {
                if let Some(command) = dashboard
                    .overwrite_chathistory_targets_timestamp(
                        clients,
                        server,
                        server_time,
                    )
                    .map(|cmd| cmd.map(Message::Dashboard))
                {
                    commands.push(command);
                }
            }
            Event::DirectMessage(encoded, our_nick, user) => {
                handle_direct_message(
                    server,
                    encoded,
                    our_nick,
                    user,
                    dashboard,
                    &mut commands,
                    clients,
                    config,
                    notifications,
                    main_window,
                );
            }
            Event::MonitoredOnline(users) => {
                let kind = history::Kind::Server(server.clone());
                let message_window = dashboard.find_window_with_history(&kind);

                if message_window.is_none() || !main_window.focused {
                    let request_attention = notifications.notify(
                        &config.notifications,
                        &Notification::MonitoredOnline(users),
                        server,
                        message_window.unwrap_or(main_window.id),
                    );

                    if let Some(request_attention) = request_attention {
                        commands.push(request_attention);
                    }
                }
            }
            Event::MonitoredOffline(users) => {
                let kind = history::Kind::Server(server.clone());
                let message_window = dashboard.find_window_with_history(&kind);

                if message_window.is_none() || !main_window.focused {
                    let request_attention = notifications.notify(
                        &config.notifications,
                        &Notification::MonitoredOffline(users),
                        server,
                        message_window.unwrap_or(main_window.id),
                    );

                    if let Some(request_attention) = request_attention {
                        commands.push(request_attention);
                    }
                }
            }
            Event::OnConnect(on_connect) => {
                let server = server.clone();
                commands.push(Task::stream(on_connect).map(move |event| {
                    Message::OnConnect(server.clone(), event)
                }));
            }
            Event::AddedIsupportParam(param) => {
                handle_isupport_param(
                    server, param, dashboard, clients, config,
                );
            }
            Event::BouncerNetwork(server, server_config) => {
                servers.insert(server, server_config.into());

                dashboard.update_filters(servers, clients, &config.buffer);
            }
            Event::AddToSidebar(query) => {
                dashboard.add_to_sidebar(server.clone(), query);
            }
            Event::Disconnect(error) => {
                for bouncer_network in servers.get_bouncer_networks(server) {
                    controllers.disconnect(bouncer_network, error.clone());
                }

                controllers.disconnect(server, error);
            }
            Event::Reaction(encoded) => {
                if let Some(reaction) = Reaction::received(
                    encoded,
                    clients.get_chantypes(server),
                    clients.get_statusmsg(server),
                    clients.get_casemapping(server),
                ) {
                    reactions.push(
                        dashboard
                            .record_reaction(server, reaction)
                            .map(Message::Dashboard),
                    );
                }
            }
        }
    }

    Task::batch(commands).chain(Task::batch(reactions))
}

fn create_message(
    server: &Server,
    encoded: message::Encoded,
    our_nick: data::user::Nick,
    config: &Config,
    clients: &data::client::Map,
) -> Option<data::Message> {
    data::Message::received(
        encoded,
        our_nick,
        config,
        |user, channel| {
            clients
                .resolve_user_attributes(server, channel, user)
                .cloned()
        },
        |channel| clients.get_channel_users(server, channel),
        server,
        clients.get_chantypes(server),
        clients.get_statusmsg(server),
        clients.get_casemapping(server),
        clients.get_prefix(server),
    )
}

fn create_message_with_highlight(
    server: &Server,
    encoded: message::Encoded,
    our_nick: data::user::Nick,
    config: &Config,
    clients: &data::client::Map,
) -> Option<(data::Message, Option<message::Highlight>)> {
    data::Message::received_with_highlight(
        encoded,
        our_nick,
        config,
        |user, channel| {
            clients
                .resolve_user_attributes(server, channel, user)
                .cloned()
        },
        |channel| clients.get_channel_users(server, channel),
        server,
        clients.get_chantypes(server),
        clients.get_statusmsg(server),
        clients.get_casemapping(server),
        clients.get_prefix(server),
    )
}

fn handle_single_event(
    server: &Server,
    encoded: message::Encoded,
    our_nick: data::user::Nick,
    dashboard: &mut screen::Dashboard,
    commands: &mut Vec<Task<Message>>,
    clients: &data::client::Map,
    config: &Config,
) {
    let Some(message) =
        create_message(server, encoded, our_nick, config, clients)
    else {
        return;
    };

    commands.push(
        dashboard
            .block_and_record_message(
                server,
                clients.get_casemapping(server),
                message,
                &config.buffer,
            )
            .map(Message::Dashboard),
    );
}

fn handle_with_target_event(
    server: &Server,
    encoded: message::Encoded,
    our_nick: data::user::Nick,
    target: message::Target,
    dashboard: &mut screen::Dashboard,
    commands: &mut Vec<Task<Message>>,
    clients: &data::client::Map,
    config: &Config,
) {
    let Some(message) =
        create_message(server, encoded, our_nick, config, clients)
    else {
        return;
    };

    commands.push(
        dashboard
            .block_and_record_message(
                server,
                clients.get_casemapping(server),
                message.with_target(target),
                &config.buffer,
            )
            .map(Message::Dashboard),
    );
}

fn handle_priv_or_notice(
    server: &Server,
    encoded: message::Encoded,
    our_nick: data::user::Nick,
    notification_enabled: bool,
    dashboard: &mut screen::Dashboard,
    commands: &mut Vec<Task<Message>>,
    clients: &mut data::client::Map,
    config: &Config,
    notifications: &mut Notifications,
    main_window: &Window,
    focused_window: Option<window::Id>,
) {
    let Some((mut msg, highlight)) = create_message_with_highlight(
        server, encoded, our_nick, config, clients,
    ) else {
        return;
    };

    let casemapping = clients.get_casemapping(server);
    let kind = history::Kind::from_server_message(server.clone(), &msg);

    if let Some(kind) = &kind {
        dashboard.block_message(
            &mut msg,
            kind,
            server,
            casemapping,
            &config.buffer,
        );
    }

    let window = kind
        .as_ref()
        .and_then(|kind| dashboard.find_window_with_history(kind));
    let should_mark_as_read = config.buffer.mark_as_read.on_message
        && !msg.blocked
        && msg.triggers_unread();

    if let Some(highlight) = highlight {
        handle_highlight(
            server,
            highlight,
            &msg,
            notification_enabled,
            window,
            casemapping,
            dashboard,
            commands,
            config,
            notifications,
            main_window,
        );
    } else {
        maybe_notify_channel_message(
            server,
            &msg,
            notification_enabled,
            window,
            casemapping,
            commands,
            config,
            notifications,
            main_window,
        );
    }

    commands.push(
        dashboard
            .record_message(server, msg, &config.buffer)
            .map(Message::Dashboard),
    );

    if should_mark_as_read && let Some(kind) = kind {
        dashboard.mark_as_read_if_focused_and_at_bottom(
            &kind,
            clients,
            focused_window,
        );
    }
}

fn handle_highlight(
    server: &Server,
    highlight: message::Highlight,
    msg: &data::Message,
    notification_enabled: bool,
    message_window: Option<window::Id>,
    casemapping: data::isupport::CaseMap,
    dashboard: &mut screen::Dashboard,
    commands: &mut Vec<Task<Message>>,
    config: &Config,
    notifications: &mut Notifications,
    main_window: &Window,
) {
    let message::Highlight {
        kind: highlight_kind,
        channel: highlight_channel,
        user: highlight_user,
        message: mut highlight_message,
    } = highlight;

    highlight_message.blocked = msg.blocked;

    if !highlight_message.blocked
        && notification_enabled
        && (message_window.is_none() || !main_window.focused)
    {
        let (description, sound) = match highlight_kind {
            message::highlight::Kind::Nick => {
                ("highlighted you".to_string(), None)
            }
            message::highlight::Kind::Match { matching, sound } => {
                (format!("matched highlight {matching}"), sound)
            }
        };

        let request_attention = notifications.notify(
            &config.notifications,
            &Notification::Highlight {
                user: highlight_user,
                channel: highlight_channel,
                casemapping,
                message: highlight_message.text(),
                description,
                sound,
            },
            server,
            message_window.unwrap_or(main_window.id),
        );

        if let Some(request_attention) = request_attention {
            commands.push(request_attention);
        }
    }

    commands.push(
        dashboard
            .record_highlight(highlight_message)
            .map(Message::Dashboard),
    );
}

fn maybe_notify_channel_message(
    server: &Server,
    msg: &data::Message,
    notification_enabled: bool,
    message_window: Option<window::Id>,
    casemapping: data::isupport::CaseMap,
    commands: &mut Vec<Task<Message>>,
    config: &Config,
    notifications: &mut Notifications,
    main_window: &Window,
) {
    if msg.blocked
        || !notification_enabled
        || (message_window.is_some() && main_window.focused)
    {
        return;
    }

    let (channel, user) = match &msg.target {
        message::Target::Channel {
            channel,
            source: message::Source::User(user),
            ..
        } => (channel.clone(), user.clone()),
        message::Target::Channel {
            channel,
            source: message::Source::Action(Some(user)),
            ..
        } => (channel.clone(), user.clone()),
        _ => return,
    };

    let request_attention = notifications.notify(
        &config.notifications,
        &Notification::Channel {
            user,
            channel,
            casemapping,
            message: msg.text(),
        },
        server,
        message_window.unwrap_or(main_window.id),
    );

    if let Some(request_attention) = request_attention {
        commands.push(request_attention);
    }
}

fn handle_broadcast(
    server: &Server,
    broadcast: data::client::Broadcast,
    dashboard: &mut screen::Dashboard,
    commands: &mut Vec<Task<Message>>,
    clients: &data::client::Map,
    config: &Config,
) {
    let casemapping = clients.get_casemapping(server);

    let task = match broadcast {
        data::client::Broadcast::Quit {
            user,
            comment,
            channels,
            sent_time,
        } => dashboard.broadcast(
            server,
            casemapping,
            config,
            sent_time,
            Broadcast::Quit {
                user,
                comment,
                user_channels: channels,
                casemapping,
            },
        ),
        data::client::Broadcast::Nickname {
            old_user,
            new_nick,
            ourself,
            channels,
            sent_time,
        } => {
            let old_nick = old_user.nickname().to_owned();
            dashboard.broadcast(
                server,
                casemapping,
                config,
                sent_time,
                Broadcast::Nickname {
                    old_nick,
                    new_nick,
                    ourself,
                    user_channels: channels,
                    casemapping,
                },
            )
        }
        data::client::Broadcast::Invite {
            inviter,
            channel,
            user_channels,
            sent_time,
        } => {
            let inviter = inviter.nickname().to_owned();
            dashboard.broadcast(
                server,
                casemapping,
                config,
                sent_time,
                Broadcast::Invite {
                    inviter,
                    channel,
                    user_channels,
                    casemapping,
                },
            )
        }
        data::client::Broadcast::ChangeHost {
            old_user,
            new_username,
            new_hostname,
            ourself,
            logged_in,
            channels,
            sent_time,
        } => dashboard.broadcast(
            server,
            casemapping,
            config,
            sent_time,
            Broadcast::ChangeHost {
                old_user,
                new_username,
                new_hostname,
                ourself,
                logged_in,
                user_channels: channels,
                casemapping,
            },
        ),
        data::client::Broadcast::Kick {
            kicker,
            victim,
            reason,
            channel,
            sent_time,
        } => dashboard.broadcast(
            server,
            casemapping,
            config,
            sent_time,
            Broadcast::Kick {
                kicker,
                victim,
                reason,
                channel,
                casemapping,
            },
        ),
    };

    commands.push(task.map(Message::Dashboard));
}

fn handle_direct_message(
    server: &Server,
    encoded: message::Encoded,
    our_nick: data::user::Nick,
    user: User,
    dashboard: &mut screen::Dashboard,
    commands: &mut Vec<Task<Message>>,
    clients: &data::client::Map,
    config: &Config,
    notifications: &mut Notifications,
    main_window: &Window,
) {
    let Some(msg) = create_message(server, encoded, our_nick, config, clients)
    else {
        return;
    };

    let chantypes = clients.get_chantypes(server);
    let statusmsg = clients.get_statusmsg(server);
    let casemapping = clients.get_casemapping(server);

    let Ok(query) = target::Query::parse(
        user.nickname().as_str(),
        chantypes,
        statusmsg,
        casemapping,
    ) else {
        return;
    };

    let blocked = FilterChain::borrow(dashboard.get_filters())
        .filter_query(&query, server);
    let kind = history::Kind::Query(server.clone(), query);

    let message_window = dashboard.find_window_with_history(&kind);

    if !blocked && (message_window.is_none() || !main_window.focused) {
        let request_attention = notifications.notify(
            &config.notifications,
            &Notification::DirectMessage {
                user,
                casemapping,
                message: msg.text(),
            },
            server,
            message_window.unwrap_or(main_window.id),
        );

        if let Some(request_attention) = request_attention {
            commands.push(request_attention);
        }
    }
}

fn handle_isupport_param(
    server: &Server,
    param: data::isupport::Parameter,
    dashboard: &mut screen::Dashboard,
    clients: &mut data::client::Map,
    config: &Config,
) {
    if matches!(param, data::isupport::Parameter::CASEMAPPING(_)) {
        dashboard.renormalize_history(server, clients);
    }

    match param {
        data::isupport::Parameter::CASEMAPPING(_)
        | data::isupport::Parameter::CHANTYPES(_) => {
            let chantypes = clients.get_chantypes(server);
            let casemapping = clients.get_casemapping(server);

            FilterChain::sync_isupport(
                dashboard.get_filters(),
                server,
                chantypes,
                casemapping,
            );
            dashboard.reprocess_history(clients, &config.buffer);
        }
        data::isupport::Parameter::SAFELIST => {
            dashboard.update_channel_discoveries(clients, server);
        }
        _ => (),
    }
}
