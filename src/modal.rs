use data::config;

use crate::widget::Element;

pub mod reload_configuration_error;

#[derive(Debug)]
pub enum Modal {
    ReloadConfigurationError(config::Error),
}

impl Modal {
    pub fn view(&self) -> Element<Close> {
        match self {
            Modal::ReloadConfigurationError(error) => reload_configuration_error::view(error),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Close;
