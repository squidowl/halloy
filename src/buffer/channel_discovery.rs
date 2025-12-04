use chrono::{Duration, Utc};
use data::{Config, Server};
use iced::widget::{
    center, column, container, pick_list, row, rule, scrollable, text, text_input,
};
use iced::{Length, Task, padding};

use crate::Theme;
use crate::appearance::theme;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    SelectServer(Server),
    SearchQuery(String),
}

pub enum Event {
    ListForServer(Server),
}

#[derive(Debug, Clone)]
pub struct ChannelDiscovery {
    selected_server: Option<Server>,
    search_query: String,
}

impl ChannelDiscovery {
    pub fn new() -> Self {
        Self {
            selected_server: None,
            search_query: String::new(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        _config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::SearchQuery(query) => {
                self.search_query = query;
                (Task::none(), None)
            }
            Message::SelectServer(server) => {
                self.selected_server = Some(server.clone());

                let should_fetch = clients
                    .get_channel_discovery_manager(&server)
                    .map(|manager| {
                        manager.last_updated.is_none()
                            || manager.last_updated.is_some_and(
                                |last_updated| {
                                    Utc::now()
                                        .signed_duration_since(last_updated)
                                        > Duration::minutes(5)
                                },
                            )
                    })
                    .unwrap_or(true);

                let event = if should_fetch {
                    Some(Event::ListForServer(server))
                } else {
                    None
                };

                (Task::none(), event)
            }
        }
    }
}

pub fn view<'a>(
    state: &'a ChannelDiscovery,
    clients: &'a data::client::Map,
    _config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let manager = state
        .selected_server
        .as_ref()
        .and_then(|server| clients.get_channel_discovery_manager(server));

    let header = container(
        column![
            row![
                pick_list(
                    clients
                        .servers()
                        .filter(|server| server.is_bouncer_network())
                        .collect::<Vec<_>>(),
                    state.selected_server.as_ref(),
                    |server: &Server| Message::SelectServer(server.clone())
                )
                .placeholder("Select server"),
                text_input("Search..", &state.search_query).on_input(|query| Message::SearchQuery(query))
            ]
            .spacing(8)
            .padding(padding::top(8)),
            container(rule::horizontal(1)).width(Length::Fill)
        ]
        .spacing(8),
    )
    .width(Length::Fill);

    let data = match manager {
        Some(manager) => container(
            scrollable(
                column(
                    manager
                        .items(&state.search_query)
                        .into_iter()
                        .map(|(channel, topic, user_count)| {
                            let has_topic = !topic.is_empty();

                            column![
                                row![
                                    text(channel),
                                    text(format!(
                                        "{} users",
                                        user_count.to_string()
                                    ))
                                    .style(theme::text::timestamp),
                                ]
                                .spacing(4),
                                has_topic.then(|| container(
                                    text(topic).style(theme::text::topic)
                                ))
                            ]
                            .into()
                        })
                        .collect::<Vec<_>>(),
                )
                .spacing(8),
            ),
        ),
        None => center(text("Select a server")),
    }
    .padding(padding::top(4))
    .width(Length::Fill)
    .height(Length::Fill);

    let content = column![header, data];

    // println!("ChannelList: {:?}", manager);
    container(content)
        .padding([0, 12])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
