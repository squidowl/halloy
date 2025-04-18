use std::path::PathBuf;

use data::{Server, config};

use crate::widget::Element;

pub mod connect_to_server;
pub mod image_preview;
pub mod prompt_before_open_url;
pub mod reload_configuration_error;

#[derive(Debug)]
pub enum Modal {
    ReloadConfigurationError(config::Error),
    ServerConnect {
        url: String,
        server: Server,
        config: config::Server,
    },
    PromptBeforeOpenUrl(String),
    ImagePreview(PathBuf),
}

#[derive(Debug, Clone)]
pub enum Message {
    Cancel,
    AcceptNewServer,
    DangerouslyAcceptInvalidCerts(bool),
    OpenURL(String),
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
            Message::OpenURL(url) => {
                let _ = open::that_detached(url);
                Some(Event::CloseModal)
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self {
            Modal::ReloadConfigurationError(error) => {
                reload_configuration_error::view(error)
            }
            Modal::ServerConnect {
                url: raw, config, ..
            } => connect_to_server::view(raw, config),
            Modal::PromptBeforeOpenUrl(payload) => {
                prompt_before_open_url::view(payload)
            }
            Modal::ImagePreview(path) => image_preview::view(path),
        }
    }
}
