use data::server::Server;
use iced::widget::{button, column, container, pane_grid, row, text};
use iced::Length;

use super::pane::Pane;
use crate::widget::Element;
use crate::{buffer, icon, theme};

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

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        panes: &pane_grid::State<Pane>,
    ) -> Element<'a, Message> {
        let mut column = column![].spacing(1);

        for (server, channels) in clients.get_channels().iter() {
            let is_channel_open = |server: &data::server::Server, channel: Option<&str>| -> bool {
                panes.iter().any(|(_, pane)| match &pane.buffer {
                    buffer::Buffer::Channel(state) => channel
                        .map(|channel| &state.server == server && state.channel == channel)
                        .unwrap_or_default(),
                    _ => false,
                })
            };

            let is_server_open = |server: &data::server::Server| -> bool {
                panes.iter().any(|(_, pane)| match &pane.buffer {
                    buffer::Buffer::Server(state) => &state.server == server,
                    _ => false,
                })
            };

            column = column.push(
                button(
                    row![icon::globe(), text(server.to_string())]
                        .spacing(8)
                        .align_items(iced::Alignment::Center),
                )
                .width(Length::Fill)
                .style(theme::Button::SideMenu {
                    selected: is_server_open(server),
                })
                .on_press(Message::SelectServer(server.clone())),
            );

            for channel in channels {
                column = column.push(
                    button(
                        row![icon::chat(), text(channel)]
                            .spacing(8)
                            .align_items(iced::Alignment::Center),
                    )
                    .width(Length::Fill)
                    .style(theme::Button::SideMenu {
                        selected: is_channel_open(server, Some(channel)),
                    })
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
