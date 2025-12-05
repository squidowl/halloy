use chrono::{Duration, Utc};
use data::{Config, Server, message, target};
use iced::widget::{
    center, column, container, pick_list, row, rule, scrollable, span, text,
    text_input,
};
use iced::{Color, Length, Task, alignment, padding};

use crate::appearance::theme;
use crate::buffer::context_menu::{self, Context};
use crate::widget::{
    Element, message_content, selectable_rich_text, selectable_text,
};
use crate::{Theme, font};

fn topic_link_entry_view<'a>(
    link: &message::Link,
    entry: context_menu::Entry,
    length: Length,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let link_context = link.url().map(Context::Url);
    entry
        .view(link_context, length, config, theme)
        .map(Message::ContextMenu)
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectServer(Server),
    SearchQuery(String),
    Join(String),
    Link(message::Link),
    ContextMenu(context_menu::Message),
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
            Message::ContextMenu(message) => {
                println!("ContextMenu: {:?}", message);
                (Task::none(), None)
            }
            Message::Link(link) => {
                println!("Link: {:?}", link);
                (Task::none(), None)
            }
            Message::Join(channel) => {
                println!("Join: {:?}", channel);
                (Task::none(), None)
            }
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
    config: &'a Config,
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
                text_input("Search..", &state.search_query)
                    .on_input(|query| Message::SearchQuery(query))
            ]
            .spacing(8)
            .padding(padding::top(8)),
            container(rule::horizontal(1)).width(Length::Fill)
        ]
        .spacing(8),
    )
    .padding(padding::horizontal(4))
    .width(Length::Fill);

    let data = match (manager, state.selected_server.as_ref()) {
        (Some(manager), Some(server)) => container(
            scrollable(
                column(
                    manager
                        .items(&state.search_query)
                        .into_iter()
                        .enumerate()
                        .map(|(idx, (channel, topic_content, user_count))| {
                            let channel_text =
                                selectable_rich_text::<
                                    _,
                                    message::Link,
                                    (),
                                    _,
                                    _,
                                >(vec![
                                    span(channel.as_str())
                                        .font_maybe(
                                            theme
                                                .styles()
                                                .buffer
                                                .url
                                                .font_style
                                                .map(font::get),
                                        )
                                        .color(theme.styles().buffer.url.color)
                                        .link(message::Link::Channel(
                                            target::Channel::from_str(
                                                channel.as_str(),
                                                clients.get_chantypes(server),
                                                clients.get_casemapping(server),
                                            ),
                                        )),
                                ])
                                .on_link(Message::Link);
                            let user_count_text = selectable_text(format!(
                                "{} users",
                                user_count.to_string()
                            ))
                            .style(theme::selectable_text::timestamp);

                            let has_topic = topic_content.text().is_empty();
                            let topic_text = if has_topic {
                                None
                            } else {
                                Some(message_content::with_context(
                                topic_content,
                                clients.get_chantypes(server),
                                clients.get_casemapping(server),
                                theme,
                                Message::Link,
                                None,
                                theme::selectable_text::topic,
                                theme::font_style::topic,
                                Option::<fn(Color) -> Color>::None,
                                move |link| match link {
                                    message::Link::Url(_) => {
                                        context_menu::Entry::url_list()
                                    }
                                    _ => vec![],
                                },
                                move |link, entry, length| {
                                    topic_link_entry_view(
                                        link, entry, length, config, theme,
                                    )
                                },
                                    config,
                                ))
                            };

                            container(column![
                                row![
                                    channel_text,
                                    selectable_text(" "),
                                    user_count_text,
                                ],
                                topic_text,
                            ])
                            .padding(padding::top(6).bottom(6).right(4).left(8))
                            .width(Length::Fill)
                            .align_y(alignment::Vertical::Center)
                            .style(move |theme| {
                                theme::container::table(theme, idx)
                            })
                            .into()
                        })
                        .collect::<Vec<_>>(),
                )
                .spacing(0),
            )
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(1).scroller_width(1),
            ))
            .style(theme::scrollable::hidden),
        ),
        _ => center(text("Select a server")),
    }
    .width(Length::Fill)
    .height(Length::Fill);

    let content = column![header, data].spacing(1).padding([0, 2]);

    // println!("ChannelList: {:?}", manager);
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
