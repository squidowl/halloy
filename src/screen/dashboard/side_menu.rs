use data::server::Server;
use iced::{
    widget::{button, column, container, horizontal_space, row, text},
    Length,
};

use crate::{theme, widget::Element};

#[derive(Debug, Clone)]
pub enum Message {
    SelectChannel((Server, String)),
    SelectServer(Server),
}

#[derive(Debug, Clone)]
pub enum Event {
    SelectChannel((Server, String)),
    SelectServer(Server),
}

#[derive(Clone)]
pub struct SideMenu {}

impl SideMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::SelectChannel((server, channel)) => {
                Some(Event::SelectChannel((server, channel)))
            }
            Message::SelectServer(server) => Some(Event::SelectServer(server)),
        }
    }

    pub fn view<'a>(&'a self, clients: &data::client::Map) -> Element<'a, Message> {
        let mut column = column![];

        for (server, channels) in clients.get_channels().iter() {
            column = column.push(
                button(text(server.to_string()))
                    .style(theme::Button::Tertiary)
                    .on_press(Message::SelectServer(server.clone())),
            );

            for channel in channels {
                column = column.push(
                    button(row![horizontal_space(Length::Fill), text(channel)])
                        .style(theme::Button::Tertiary)
                        .on_press(Message::SelectChannel((server.clone(), channel.clone()))),
                );
            }
        }

        container(column)
            .padding([8, 0, 6, 6])
            .center_x()
            .max_width(120)
            .into()
    }
}
