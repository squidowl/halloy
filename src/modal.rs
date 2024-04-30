use data::config;

use crate::widget::Element;

pub mod reload_configuration_error;
pub mod url_route_received;

#[derive(Debug)]
pub enum Modal {
    ReloadConfigurationError(config::Error),
    UrlRouteReceived(ipc::Route),
}

impl Modal {
    pub fn view(&self) -> Element<Message> {
        match self {
            Modal::ReloadConfigurationError(error) => reload_configuration_error::view(error),
            Modal::UrlRouteReceived(route) => url_route_received::view(route),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Cancel,
    Accept,
}
