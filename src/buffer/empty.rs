use core::fmt;

use data::{config, Config};
use iced::widget::{button, column, container, text, vertical_space};
use iced::{alignment, Length};

use crate::theme;
use crate::widget::{Collection, Element};

#[derive(Debug, Clone)]
pub enum Message {
    ConfigurationDirectoryPressed,
}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(
    _state: &Empty,
    clients: &data::client::Map,
    // TODO: Make error a separate screen so we don't
    // have to pass this all the way down
    load_config_error: &'a Option<config::Error>,
) -> Element<'a, Message> {
    let is_empty = clients.get_channels().is_empty();
    let config_dir = is_empty
        .then(|| {
            Config::config_dir()
                .map(|path| String::from(path.to_string_lossy()))
                .ok()
        })
        .flatten();

    let error = load_config_error
        .as_ref()
        .map(|error| text(error.to_string()).style(theme::Text::Error));
    let title = if is_empty {
        text("please create or edit config.yaml in the directory below")
    } else {
        text("you had me at halloy")
    };

    let action = config_dir.map(|path| {
        button(text(path))
            .on_press(Message::ConfigurationDirectoryPressed)
            .style(theme::Button::Default)
    });
    let content = column![]
        .push_maybe(error)
        .push(title)
        .push(vertical_space(14))
        .push_maybe(action)
        .align_items(iced::Alignment::Center);

    container(content)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[derive(Debug, Default, Clone)]
pub struct Empty {}

impl Empty {
    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::ConfigurationDirectoryPressed => {
                let Ok(config) = Config::config_dir() else {
                    return None
                };

                let _ = open::that(config);
            }
        }

        None
    }
}

impl fmt::Display for Empty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}
