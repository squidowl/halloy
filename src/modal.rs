use data::config;

use crate::widget::Element;

pub mod reload_configuration_error;

#[derive(Debug)]
pub enum Modal {
    ReloadConfigurationError(config::Error),
}

impl Modal {
    pub fn view(&self) -> Element<Message> {
        match self {
            Modal::ReloadConfigurationError(error) => {
                reload_configuration_error::view(error).map(Message::ReloadConfigurationError)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ReloadConfigurationError(reload_configuration_error::Message),
}
