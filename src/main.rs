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

        for server in &config.servers {
            clients.disconnected(server.server.clone().expect("config server").into());
        }

        (
            Halloy {
                screen: Screen::Dashboard(screen),
                theme: Theme::new_from_palette(config.palette),
                config,
                clients,
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
                    let (command, event) = dashboard.update(message, &mut self.clients);

                    match event {
                        Some(event) => match event {},
                        None => {}
                    }

                    command.map(Message::Dashboard)
                }
            },
            Message::Stream(Ok(event)) => match event {
                stream::Event::Ready(sender) => {
                    log::debug!("Client ready to receive connections");

                    for server in self.config.servers.clone() {
                        let _ = sender.blocking_send(stream::Message::Connect(server));
                    }

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
        }
    }

    fn view(&self) -> Element<Message> {
        let content = match &self.screen {
            Screen::Dashboard(dashboard) => dashboard.view(&self.clients).map(Message::Dashboard),
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
