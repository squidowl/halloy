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
use crate::{Theme, font, icon};

#[derive(Debug, Clone)]
pub enum Message {
    SelectServer(Server),
    SearchQuery(String),
    Link(message::Link),
    ContextMenu(context_menu::Message),
}

pub enum Event {
    ListForServer(Server),
    OpenUrl(String),
    OpenChannelForServer(data::Server, target::Channel),
}

#[derive(Debug, Clone, Default)]
pub struct ChannelDiscovery {
    pub server: Option<Server>,
    pub search_query: String,
}

impl ChannelDiscovery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        _config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ContextMenu(message) => {
                println!("ContextMenu: {message:?}");
                (Task::none(), None)
            }
            Message::Link(link) => match link {
                message::Link::Url(url) => {
                    (Task::none(), Some(Event::OpenUrl(url)))
                }
                message::Link::Channel(channel) => {
                    if let Some(server) = self.server.clone() {
                        (
                            Task::none(),
                            Some(Event::OpenChannelForServer(server, channel)),
                        )
                    } else {
                        (Task::none(), None)
                    }
                }
                _ => (Task::none(), None),
            },
            Message::SearchQuery(query) => {
                self.search_query = query;
                (Task::none(), None)
            }
            Message::SelectServer(server) => {
                self.server = Some(server.clone());

                let should_fetch = clients
                    .get_channel_discovery_manager(&server)
                    .is_none_or(|manager| {
                        manager.last_updated.is_none()
                            || manager.last_updated.is_some_and(
                                |last_updated| {
                                    Utc::now()
                                        .signed_duration_since(last_updated)
                                        > Duration::minutes(5)
                                },
                            )
                    });

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
        .server
        .as_ref()
        .and_then(|server| clients.get_channel_discovery_manager(server));

    let selected_server = state.server.as_ref();

    let header = container(
        column![
            row![
                pick_list(
                    clients.servers().collect::<Vec<_>>(),
                    selected_server,
                    |server: &Server| Message::SelectServer(server.clone())
                )
                .placeholder("Select server"),
                text_input("Search..", &state.search_query).on_input_maybe(
                    selected_server
                        .map(|_| |query| Message::SearchQuery(query))
                ),
            ]
            .spacing(8)
            .padding(padding::top(8)),
            container(rule::horizontal(1)).width(Length::Fill)
        ]
        .spacing(8),
    )
    .padding(padding::horizontal(4))
    .width(Length::Fill);

    let data = match (manager, selected_server) {
        (Some(manager), Some(server)) => {
            let items = manager.items(&state.search_query);
            if items.is_empty() {
                container(center(
                    text("No channels found")
                        .style(theme::text::secondary)
                        .font_maybe(theme::font_style::secondary(theme).map(font::get)),
                ))
            } else {
                container(
                    scrollable(
                        column(
                            items
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
                                        "{user_count} users"
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
                                                entry.view(link.url().map(Context::Url), length, config, theme).map(Message::ContextMenu)
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
                        scrollable::Scrollbar::default()
                            .width(config.pane.scrollbar.width)
                            .scroller_width(config.pane.scrollbar.scroller_width),
                    ))
                )
            }
        },
        _ => container(center(
            column![
                icon::channel_discovery()
                    .size(theme::TEXT_SIZE + 3.0)
                    .style(theme::text::secondary),
                text("Select a server")
                    .style(theme::text::secondary)
                    .font_maybe(
                        theme::font_style::secondary(theme).map(font::get)
                    ),
            ]
            .spacing(8)
            .align_x(iced::Alignment::Center)
        )),
    }
    .width(Length::Fill)
    .height(Length::Fill);

    let content = column![header, data].spacing(1).padding([0, 2]);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
