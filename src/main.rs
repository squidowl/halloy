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
use iced::{executor, keyboard, subscription, Application, Command, Length, Subscription};
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

fn settings() -> iced::Settings<()> {
    iced::Settings {
        default_font: font::MONO,
        default_text_size: theme::TEXT_SIZE,
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
}

impl Application for Halloy {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = theme::Theme;

    fn new(_flags: ()) -> (Halloy, Command<Self::Message>) {
        let config = Config::load().unwrap_or_default();
        let screen = screen::Dashboard::new(&config);

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
            Command::batch(vec![font::load().map(Message::FontsLoaded)]),
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

                    command.map(Message::Dashboard)
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
                stream::Event::MessageReceived(server, message) => {
                    let Some(source) = self.clients.add_message(&server, message) else {
                        return Command::none();
                    };
                    let Screen::Dashboard(dashboard) = &self.screen;

                    dashboard
                        .message_received(&server, source)
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
            Message::Event(event) => {
                let message = match event {
                    iced::Event::Keyboard(keyboard) => match keyboard {
                        keyboard::Event::KeyPressed {
                            key_code,
                            modifiers,
                        } => match &self.screen {
                            Screen::Dashboard(state) => state
                                .handle_keypress(key_code, modifiers)
                                .map(Message::Dashboard),
                        },
                        keyboard::Event::KeyReleased { .. } => None,
                        keyboard::Event::CharacterReceived(_) => None,
                        keyboard::Event::ModifiersChanged(_) => None,
                    },
                    _ => None,
                };

                if let Some(message) = message {
                    return self.update(message);
                }

                Command::none()
            }
            Message::ConfigSaved(Ok(_)) => Command::none(),
            Message::ConfigSaved(Err(error)) => {
                log::error!("config saved failed: {error:?}");
                Command::none()
            }
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
        Subscription::batch(vec![
            client::run().map(Message::Stream),
            subscription::events_with(filtered_events),
        ])
    }
}

// Always capture ESC to unfocus pane right away
fn filtered_events(event: iced::Event, status: iced::event::Status) -> Option<Message> {
    use iced::event;

    if let iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key_code: keyboard::KeyCode::Escape,
        ..
    }) = &event
    {
        Some(Message::Event(event))
    } else {
        matches!(status, event::Status::Ignored).then_some(Message::Event(event))
    }
}
