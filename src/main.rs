#![allow(clippy::large_enum_variant, clippy::too_many_arguments)]

mod buffer;
mod client;
mod font;
mod icon;
mod logger;
mod screen;
mod theme;
mod widget;

use data::config::Config;
use data::stream;
use iced::widget::container;
use iced::{executor, keyboard, subscription, window, Application, Command, Length, Subscription};
use tokio::sync::mpsc;

use self::screen::dashboard;
pub use self::theme::Theme;
use self::widget::Element;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn main() -> iced::Result {
    #[cfg(debug_assertions)]
    let is_debug = true;
    #[cfg(not(debug_assertions))]
    let is_debug = false;

    logger::setup(is_debug).expect("setup logging");
    log::info!("application ({}) has started", VERSION);

    if let Err(error) = Halloy::run(settings()) {
        log::error!("{}", error.to_string());
        Err(error)
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
fn window_settings() -> iced::window::Settings {
    Default::default()
}

#[cfg(target_os = "macos")]
fn window_settings() -> iced::window::Settings {
    iced::window::Settings {
        platform_specific: iced::window::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        ..Default::default()
    }
}

fn settings() -> iced::Settings<()> {
    iced::Settings {
        default_font: font::MONO,
        default_text_size: theme::TEXT_SIZE,
        window: iced::window::Settings {
            ..window_settings()
        },
        exit_on_close_request: false,
        ..Default::default()
    }
}

struct Halloy {
    screen: Screen,
    theme: Theme,
    config: Config,
    clients: data::client::Map,
    stream: Option<mpsc::Sender<stream::Message>>,
}

impl Halloy {
    fn config(&self) -> Config {
        Config {
            palette: self.config.palette,
            servers: self.config.servers.clone(),
            channels: self.config.channels.clone(),
            user_colors: self.config.user_colors.clone(),
            error: self.config.error.clone(),
        }
    }
}

enum Screen {
    Dashboard(screen::Dashboard),
}

#[derive(Debug)]
enum Message {
    Dashboard(dashboard::Message),
    Stream(stream::Result),
    Event(iced::Event),
    FontsLoaded(Result<(), iced::font::Error>),
    ConfigSaved(Result<(), data::config::Error>),
    Exit,
}

impl Application for Halloy {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = theme::Theme;

    fn new(_flags: ()) -> (Halloy, Command<Self::Message>) {
        let config = Config::load();
        let (screen, command) = screen::Dashboard::new();

        let mut clients = data::client::Map::default();

        for (server, server_config) in &config.servers {
            let server = data::Server::new(
                server,
                server_config.server.as_ref().expect("server hostname"),
            );
            clients.disconnected(server);
        }

        (
            Halloy {
                screen: Screen::Dashboard(screen),
                theme: Theme::new_from_palette(config.palette),
                config,
                clients,
                stream: None,
            },
            Command::batch(vec![
                font::load().map(Message::FontsLoaded),
                command.map(Message::Dashboard),
            ]),
        )
    }

    fn title(&self) -> String {
        String::from("Halloy")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Dashboard(message) => match &mut self.screen {
                Screen::Dashboard(dashboard) => {
                    let (command, event) =
                        dashboard.update(message, &mut self.clients, &mut self.config);
                    // Retrack after dashboard state changes
                    let track = dashboard.track();

                    if let Some(event) = event {
                        match event {
                            dashboard::Event::SaveSettings => {
                                return Command::perform(
                                    self.config().save(),
                                    Message::ConfigSaved,
                                );
                            }
                        }
                    }

                    Command::batch(vec![
                        command.map(Message::Dashboard),
                        track.map(Message::Dashboard),
                    ])
                }
            },
            Message::Stream(Ok(event)) => match event {
                stream::Event::Ready(sender) => {
                    log::debug!("Client ready to receive connections");

                    for (name, config) in self.config.servers.clone() {
                        let _ = sender.blocking_send(stream::Message::Connect(name, config));
                    }

                    // Hold this to prevent the channel from closing and
                    // putting stream into a loop
                    self.stream = Some(sender);

                    Command::none()
                }
                stream::Event::Connected(server, client) => {
                    log::info!("Connected to {:?}", server);
                    self.clients.ready(server, client);

                    Command::none()
                }
                stream::Event::MessagesReceived(messages) => {
                    let Screen::Dashboard(dashboard) = &mut self.screen;
                    dashboard
                        .messages_received(messages)
                        .map(Message::Dashboard)
                }
            },
            Message::Stream(Err(error)) => {
                log::error!("{:?}", error);
                Command::none()
            }
            Message::FontsLoaded(Ok(())) => Command::none(),
            Message::FontsLoaded(Err(error)) => {
                log::error!("fonts failed to load: {error:?}");
                Command::none()
            }
            Message::Event(event) => match event {
                iced::Event::Keyboard(keyboard) => match keyboard {
                    keyboard::Event::KeyPressed {
                        key_code,
                        modifiers,
                    } => match &mut self.screen {
                        Screen::Dashboard(state) => state
                            .handle_keypress(key_code, modifiers)
                            .map(Message::Dashboard),
                    },
                    keyboard::Event::KeyReleased { .. } => Command::none(),
                    keyboard::Event::CharacterReceived(_) => Command::none(),
                    keyboard::Event::ModifiersChanged(_) => Command::none(),
                },
                iced::Event::Window(window::Event::CloseRequested) => {
                    let Screen::Dashboard(dashboard) = &mut self.screen;
                    dashboard.exit().map(|_| Message::Exit)
                }
                _ => Command::none(),
            },
            Message::ConfigSaved(Ok(_)) => Command::none(),
            Message::ConfigSaved(Err(error)) => {
                log::error!("config saved failed: {error:?}");
                Command::none()
            }
            Message::Exit => window::close(),
        }
    }

    fn view(&self) -> Element<Message> {
        let content = match &self.screen {
            Screen::Dashboard(dashboard) => dashboard
                .view(&self.clients, &self.config)
                .map(Message::Dashboard),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Primary)
            .into()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Message> {
        let Screen::Dashboard(dashboard) = &self.screen;

        Subscription::batch(vec![
            client::run().map(Message::Stream),
            subscription::events_with(filtered_events),
            dashboard.subscription().map(Message::Dashboard),
        ])
    }
}

// Always capture ESC to unfocus pane right away
fn filtered_events(event: iced::Event, status: iced::event::Status) -> Option<Message> {
    use iced::event;

    match &event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key_code: keyboard::KeyCode::Escape,
            ..
        }) => Some(Message::Event(event)),
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key_code: keyboard::KeyCode::C,
            modifiers,
        }) if modifiers.command() => Some(Message::Event(event)),
        iced::Event::Keyboard(_) if matches!(status, event::Status::Ignored) => {
            Some(Message::Event(event))
        }
        iced::Event::Window(window::Event::CloseRequested) => Some(Message::Event(event)),
        _ => None,
    }
}
