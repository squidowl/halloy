mod client;
mod config;
mod icon;
mod logger;
mod screen;
mod style;
mod theme;

use client::Client;
use config::Config;
use iced::{
    executor,
    pure::{container, Application, Element},
    Command, Length, Settings, Subscription,
};
use screen::dashboard;
use theme::Theme;

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
    client: Client,
}

enum Screen {
    Dashboard(screen::Dashboard),
}

#[derive(Debug)]
enum Message {
    Dashboard(dashboard::Message),
    ConfigSaved(Result<(), config::Error>),
    // TODO: Change to own error.
    ClientSetup(irc::error::Result<Client>),
    ClientMessageReceived((irc::client::Sender, irc::proto::Message)),
}

impl Application for Halloy {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Halloy, Command<Self::Message>) {
        let screen = screen::Dashboard::new();
        let config = Config::load().unwrap_or_default();
        let servers = config.servers.clone();

        (
            Halloy {
                screen: Screen::Dashboard(screen),
                theme: Theme::default(),
                client: Client::default(),
            },
            Command::batch(vec![Command::perform(
                Client::setup(servers),
                Message::ClientSetup,
            )]),
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
            Message::ConfigSaved(_) => {
                log::info!("config saved to disk");
            }
            Message::ClientSetup(result) => match result {
                Ok(client) => {
                    self.client = client;
                }
                Err(error) => {
                    log::error!("{:?}", error);
                }
            },
            Message::ClientMessageReceived((sender, message)) => {}
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
        // Subscription::batch(vec![self
        //     .client
        //     .on_message()
        //     .map(Message::ClientMessageReceived)])

        Subscription::none()
    }
}
