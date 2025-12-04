use data::{Config, Server, channel_list};
use iced::widget::{column, container, pick_list, row, rule, scrollable, text};
use iced::{Length, Task, padding};

use crate::Theme;
use crate::appearance::theme;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    SelectServer(Server),
}

pub enum Event {
    ListForServer(Server),
}

#[derive(Debug, Clone)]
pub struct ChannelList {
    selected_server: Option<Server>,
}

impl ChannelList {
    pub fn new() -> Self {
        Self {
            selected_server: None,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        _config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::SelectServer(server) => {
                self.selected_server = Some(server.clone());

                (Task::none(), Some(Event::ListForServer(server)))
            }
        }
    }
}

pub fn view<'a>(
    state: &'a ChannelList,
    clients: &'a data::client::Map,
    _config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let manager = state.selected_server.as_ref().and_then(|server| clients.get_channel_list(server));

    let header = container(
        column![
            row![
                pick_list(
                    // TODO: Filter bouncer servers away.
                    clients.servers().collect::<Vec<_>>(),
                    state.selected_server.as_ref(),
                    |server: &Server| Message::SelectServer(server.clone())
                )
                .placeholder("Select server")
            ]
            .padding(padding::top(8)),
            container(rule::horizontal(1)).width(Length::Fill)
        ]
        .spacing(8),
    )
    .width(Length::Fill);

    let data = match manager {
        Some(manager) => {
            container(column(manager.items().map(|(channel, topic, user_count)| {
                column![
                    row![
                        text(user_count.to_string()).style(theme::text::timestamp).width(Length::Fixed(20.0)),
                        text(channel)
                    ]
                    .spacing(8),
                    container(text(topic).style(theme::text::topic)).padding(padding::left(20))
                ]
                .into()
            })))
        }
        None => container(text("Select a server")),
    };

    let content = column![header, scrollable(data),];

    // println!("ChannelList: {:?}", manager);
    container(content)
        .padding([0, 12])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
