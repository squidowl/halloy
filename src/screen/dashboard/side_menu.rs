use data::server::Server;
use data::{history, User};
use iced::widget::{button, column, container, horizontal_space, pane_grid, row, text};
use iced::Length;

use super::pane::Pane;
use crate::widget::Element;
use crate::{buffer, icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Channel((Server, String)),
    Server(Server),
    Query((Server, User)),
}

#[derive(Debug, Clone)]
pub enum Event {
    Channel((Server, String)),
    Server(Server),
    Query((Server, User)),
}

#[derive(Clone)]
pub struct SideMenu {}

impl SideMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::Channel((server, channel)) => Some(Event::Channel((server, channel))),
            Message::Server(server) => Some(Event::Server(server)),
            Message::Query((server, user)) => Some(Event::Query((server, user))),
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        history: &'a history::Manager,
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

            let is_query_open = |server: &data::server::Server, user: Option<&User>| -> bool {
                panes.iter().any(|(_, pane)| match &pane.buffer {
                    buffer::Buffer::Query(state) => user
                        .map(|user| &state.server == server && &state.user == user)
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
                .on_press(Message::Server(server.clone())),
            );

            for channel in channels {
                column = column.push(
                    button(
                        row![horizontal_space(4), icon::chat(), text(channel)]
                            .spacing(8)
                            .align_items(iced::Alignment::Center),
                    )
                    .width(Length::Fill)
                    .style(theme::Button::SideMenu {
                        selected: is_channel_open(server, Some(channel)),
                    })
                    .on_press(Message::Channel((server.clone(), channel.clone()))),
                );
            }

            let queries = history.get_unique_queries(server);
            for user in queries {
                column = column.push(
                    button(
                        row![horizontal_space(4), icon::person(), text(user.nickname())]
                            .spacing(8)
                            .align_items(iced::Alignment::Center),
                    )
                    .width(Length::Fill)
                    .style(theme::Button::SideMenu {
                        selected: is_query_open(server, Some(user)),
                    })
                    .on_press(Message::Query((server.clone(), user.clone()))),
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
