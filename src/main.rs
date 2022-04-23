mod buffer;
mod client;
mod config;
mod icon;
mod logger;
mod screen;
mod style;
mod theme;

use std::collections::HashMap;

use config::Config;
use iced::{
    executor,
    pure::{container, Application, Element},
    Command, Length, Settings, Subscription,
};
use screen::dashboard;
use theme::Theme;
use tokio::sync::mpsc;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn main() -> iced::Result {
    #[cfg(debug_assertions)]
    let is_debug = true;
    #[cfg(not(debug_assertions))]
    let is_debug = false;

    logger::setup(is_debug).expect("setup logging");
    log::info!("application ({}) has started", VERSION);

    if let Err(error) = Halloy::run(Settings::default()) {
        log::error!("{}", error.to_string());
        Err(error)
    } else {
        Ok(())
    }
}

struct Halloy {
    screen: Screen,
    theme: Theme,
    config: Config,
    sender: Option<mpsc::Sender<client::Message>>,
    servers: HashMap<String, irc::client::Sender>,
}

enum Screen {
    Dashboard(screen::Dashboard),
}

#[derive(Debug)]
enum Message {
    Dashboard(dashboard::Message),
    Client(client::Result),
}

impl Application for Halloy {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Halloy, Command<Self::Message>) {
        let config = Config::load().unwrap_or_default();
        let screen = screen::Dashboard::new(&config);

        (
            Halloy {
                screen: Screen::Dashboard(screen),
                theme: Theme::default(),
                config,
                sender: None,
                servers: Default::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Halloy")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Dashboard(message) => match &mut self.screen {
                Screen::Dashboard(dashboard) => {
                    if let Some((_event, _command)) = dashboard.update(message) {
                        // Handle events and commands.
                    }
                }
            },
            Message::Client(Ok(event)) => match event {
                client::Event::Ready(sender) => {
                    log::debug!("Client ready to receive connections");

                    for server in self.config.servers.clone() {
                        let _ = sender.blocking_send(client::Message::Connect(server));
                    }

                    self.sender = Some(sender);
                }
                client::Event::Connected(server, sender) => {
                    log::info!("connected to {:?}", server);
                    self.servers.insert(server, sender);
                }
                client::Event::MessageReceived(server, message) => {
                    log::debug!("Server {} message received: {:?}", server, message);
                }
            },
            Message::Client(Err(error)) => {
                log::error!("{:?}", error);
            }
        }

        Command::none()
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        let content = match &self.screen {
            Screen::Dashboard(dashboard) => dashboard.view(&self.theme).map(Message::Dashboard),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        client::run().map(Message::Client)
    }
}
