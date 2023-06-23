#![allow(clippy::large_enum_variant, clippy::too_many_arguments)]

mod buffer;
mod event;
mod font;
mod icon;
mod logger;
mod screen;
mod stream;
mod theme;
mod widget;

use data::config::{self, Config};
use iced::widget::container;
use iced::{executor, Application, Command, Length, Subscription};

use self::event::{events, Event};
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
    // TODO: Make this a different screen?
    load_config_error: Option<config::Error>,
}

enum Screen {
    Dashboard(screen::Dashboard),
}

#[derive(Debug)]
enum Message {
    Dashboard(dashboard::Message),
    Stream(stream::Update),
    Event(Event),
    FontsLoaded(Result<(), iced::font::Error>),
}

impl Application for Halloy {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = theme::Theme;

    fn new(_flags: ()) -> (Halloy, Command<Self::Message>) {
        let (config, load_config_error) = match Config::load() {
            Ok(config) => (config, None),
            Err(error) => (Config::default(), Some(error)),
        };
        let (screen, command) = screen::Dashboard::new(&config);

        (
            Halloy {
                screen: Screen::Dashboard(screen),
                theme: Theme::new_from_palette(config.palette),
                config,
                clients: Default::default(),
                load_config_error,
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
                    let command = dashboard.update(message, &mut self.clients, &self.config);
                    // Retrack after dashboard state changes
                    let track = dashboard.track();

                    Command::batch(vec![
                        command.map(Message::Dashboard),
                        track.map(Message::Dashboard),
                    ])
                }
            },
            Message::Stream(update) => match update {
                stream::Update::Disconnected { server, is_initial } => {
                    self.clients.disconnected(server.clone());

                    if !is_initial {
                        let Screen::Dashboard(dashboard) = &mut self.screen;
                        dashboard.disconnected(&server);
                    }

                    Command::none()
                }
                stream::Update::Connected {
                    server,
                    connection,
                    is_initial,
                } => {
                    self.clients.ready(server.clone(), connection);

                    if !is_initial {
                        let Screen::Dashboard(dashboard) = &mut self.screen;
                        dashboard.reconnected(&server);
                    }

                    Command::none()
                }
                stream::Update::MessagesReceived(server, messages) => {
                    let Screen::Dashboard(dashboard) = &mut self.screen;

                    messages.into_iter().for_each(|encoded| {
                        if let Some(message) = self.clients.receive(&server, encoded) {
                            dashboard.record_message(&server, message);
                        }
                    });

                    Command::none()
                }
            },
            Message::FontsLoaded(Ok(())) => Command::none(),
            Message::FontsLoaded(Err(error)) => {
                log::error!("fonts failed to load: {error:?}");
                Command::none()
            }
            Message::Event(event) => {
                let Screen::Dashboard(dashboard) = &mut self.screen;
                dashboard.handle_event(event).map(Message::Dashboard)
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let content = match &self.screen {
            Screen::Dashboard(dashboard) => dashboard
                .view(&self.clients, &self.load_config_error)
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

        let streams = Subscription::batch(self.config.servers.entries().map(stream::run))
            .map(Message::Stream);

        Subscription::batch(vec![
            streams,
            events().map(Message::Event),
            dashboard.subscription().map(Message::Dashboard),
        ])
    }
}
