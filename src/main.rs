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
use iced::{executor, widget::container, Application, Command, Length, Subscription};
use screen::dashboard;
use theme::Theme;
use widget::Element;

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
                    if let Some((_event, _command)) = dashboard.update(message, &mut self.clients) {
                        // Handle events and commands.
                    }

                    Command::none()
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
                    log::info!("connected to {:?}", server);
                    self.clients.ready(server, client);

                    Command::none()
                }
                stream::Event::MessageReceived(server, message) => {
                    // log::debug!("Server {:?} message received: {:?}", &server, &message);
                    self.clients.add_message(&server, message);

                    Command::none()
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

    fn subscription(&self) -> Subscription<Message> {
        client::run().map(Message::Stream)
    }
}
