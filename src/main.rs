mod buffer;
mod client;
mod font;
mod icon;
mod logger;
mod screen;
mod style;
mod widget;

use data::config::Config;
use data::stream;
use data::theme::Theme;
use iced::{
    executor,
    pure::{container, Application, Element},
    Command, Length, Subscription,
};
use screen::dashboard;

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
        default_font: Some(include_bytes!("../fonts/iosevka-term-regular.ttf")),
        default_text_size: style::TEXT_SIZE,
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
}

impl Application for Halloy {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

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
                theme: Theme::default(),
                config,
                clients,
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
                    if let Some((_event, _command)) = dashboard.update(message, &mut self.clients) {
                        // Handle events and commands.
                    }
                }
            },
            Message::Stream(Ok(event)) => match event {
                stream::Event::Ready(sender) => {
                    log::debug!("Client ready to receive connections");

                    for server in self.config.servers.clone() {
                        let _ = sender.blocking_send(stream::Message::Connect(server));
                    }
                }
                stream::Event::Connected(server, client) => {
                    log::info!("connected to {:?}", server);
                    self.clients.ready(server, client);
                }
                stream::Event::MessageReceived(server, message) => {
                    // log::debug!("Server {:?} message received: {:?}", &server, &message);
                    self.clients.add_message(&server, message);
                }
            },
            Message::Stream(Err(error)) => {
                log::error!("{:?}", error);
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let content = match &self.screen {
            Screen::Dashboard(dashboard) => dashboard
                .view(&self.clients, &self.theme)
                .map(Message::Dashboard),
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        client::run().map(Message::Stream)
    }
}
