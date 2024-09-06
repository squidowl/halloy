use crate::widget::Element;
use data::{config, Server};

pub mod connect_to_server;
pub mod reload_configuration_error;

#[derive(Debug)]
pub enum Modal {
    ReloadConfigurationError(config::Error),
    ServerConnect {
        url: String,
        server: Server,
        config: config::Server,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Cancel,
    AcceptNewServer,
    DangerouslyAcceptInvalidCerts(bool),
}

pub enum Event {
    CloseModal,
    AcceptNewServer,
}

impl Modal {
    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::Cancel => Some(Event::CloseModal),
            Message::AcceptNewServer => Some(Event::AcceptNewServer),
            Message::DangerouslyAcceptInvalidCerts(toggle) => {
                if let Modal::ServerConnect { config, .. } = self {
                    config.dangerously_accept_invalid_certs = toggle;
                }

                None
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self {
            Modal::ReloadConfigurationError(error) => reload_configuration_error::view(error),
            Modal::ServerConnect {
                url: raw, config, ..
            } => connect_to_server::view(raw, config),
        }
    }
}
