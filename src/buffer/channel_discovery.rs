use data::{Config, Server, channel_discovery, message, target};
use iced::widget::{
    self, button, center, column, container, operation, pick_list, row, rule,
    scrollable, span, text, text_input,
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
    SendUnsafeList(Server),
}

pub enum Event {
    SelectedServer(Server),
    OpenUrl(String),
    OpenChannelForServer(data::Server, target::Channel),
    ContextMenu(context_menu::Event),
    SendUnsafeList(Server),
}

#[derive(Debug, Clone)]
pub struct ChannelDiscovery {
    pub server: Option<Server>,
    pub search_query: String,
    search_query_id: widget::Id,
}

impl Default for ChannelDiscovery {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ChannelDiscovery {
    pub fn new(server: Option<Server>) -> Self {
        Self {
            server,
            search_query: String::new(),
            search_query_id: widget::Id::unique(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        _config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ContextMenu(message) => (
                Task::none(),
                Some(Event::ContextMenu(context_menu::update(message))),
            ),
            Message::Link(link) => match link {
                message::Link::Url(url) => {
                    (Task::none(), Some(Event::OpenUrl(url)))
                }
                message::Link::Channel(server, channel) => (
                    Task::none(),
                    Some(Event::OpenChannelForServer(server, channel)),
                ),
                _ => (Task::none(), None),
            },
            Message::SearchQuery(query) => {
                self.search_query = query;
                (Task::none(), None)
            }
            Message::SelectServer(server) => {
                self.server = Some(server.clone());

                (Task::none(), Some(Event::SelectedServer(server)))
            }
            Message::SendUnsafeList(server) => {
                (Task::none(), Some(Event::SendUnsafeList(server)))
            }
        }
    }

    pub fn focus(&self) -> Task<Message> {
        let search_query_id = self.search_query_id.clone();

        operation::is_focused(search_query_id.clone()).then(move |is_focused| {
            if is_focused {
                Task::none()
            } else {
                operation::focus(search_query_id.clone())
            }
        })
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

    let chantypes = clients.get_chantypes_or_default(state.server.as_ref());

    let selected_server = state.server.as_ref();

    let header = container(
        column![
            row![
                pick_list(
                    selected_server,
                    clients.servers().cloned().collect::<Vec<_>>(),
                    Server::to_string
                )
                .on_select(Message::SelectServer)
                .placeholder("Select server"),
                text_input("Search..", &state.search_query)
                    .id(state.search_query_id.clone())
                    .style(move |theme, status| {
                        // Show the disabled text_input as active, since we only
                        // expect it to be disabled when moving panes (and that
                        // disabling does not need to be indicated to the user)
                        if matches!(status, text_input::Status::Disabled) {
                            theme::text_input::primary(
                                theme,
                                text_input::Status::Active,
                            )
                        } else {
                            theme::text_input::primary(theme, status)
                        }
                    })
                    .on_input_maybe(
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
            let items = manager.items(&state.search_query, chantypes);
            if items.is_empty() {
                if clients.get_server_is_connected(server)
                    && !clients.get_server_supports_list(server)
                {
                    container(center(unsafe_list_view(server, theme)))
                } else {
                    let reason = if !clients.get_server_is_connected(server) {
                        "Disconnected from server"
                    } else {
                        match manager.status {
                            Some(channel_discovery::Status::Updated(_)) => {
                                "No channels found"
                            }
                            _ => "...",
                        }
                    };

                    container(center(
                        text(reason).style(theme::text::secondary).font_maybe(
                            theme::font_style::secondary(theme).map(font::get),
                        ),
                    ))
                }
            } else {
                container(channel_list_view(
                    items, server, clients, config, theme,
                ))
            }
        }
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
            .align_x(iced::Alignment::Center),
        )),
    }
    .width(Length::Fill)
    .height(Length::Fill);

    let content = column![header, data].spacing(1).padding([2, 2]);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn channel_list_view<'a>(
    items: Vec<(&'a String, &'a message::Content, &'a usize)>,
    server: &'a Server,
    clients: &'a data::client::Map,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    scrollable(
        column(
            items
                .into_iter()
                .enumerate()
                .map(|(idx, (channel, topic_content, user_count))| {
                    let channel_text =
                        selectable_rich_text::<_, message::Link, (), _, _>(
                            vec![
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
                                        server.clone(),
                                        target::Channel::from_str(
                                            channel.as_str(),
                                            clients.get_chantypes(server),
                                            clients.get_casemapping(server),
                                        ),
                                    )),
                            ],
                        )
                        .on_link(Message::Link);
                    let user_count_text =
                        selectable_text(format!("{user_count} users"))
                            .style(theme::selectable_text::timestamp);

                    let has_topic = topic_content.text().is_empty();
                    let topic_text = if has_topic {
                        None
                    } else {
                        Some(message_content::with_context(
                            topic_content,
                            server,
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
                                    context_menu::Entry::url_list(None)
                                }
                                _ => vec![],
                            },
                            move |link, entry, length| {
                                entry
                                    .view(
                                        link.url().map(|url| Context::Url {
                                            url,
                                            message: None,
                                        }),
                                        length,
                                        config,
                                        theme,
                                    )
                                    .map(Message::ContextMenu)
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
                    .style(move |theme| theme::container::table(theme, idx))
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
    .into()
}

fn unsafe_list_view<'a>(
    server: &'a Server,
    theme: &'a Theme,
) -> Element<'a, Message> {
    column![
        text(
            "Server does not register SAFELIST support\n\
             Requesting channels may disconnect you"
        )
        .align_x(alignment::Horizontal::Center)
        .style(theme::text::secondary)
        .font_maybe(theme::font_style::secondary(theme).map(font::get)),
        button(text("Fetch Channels"))
            .style(|theme, status| theme::button::secondary(
                theme, status, false
            ))
            .on_press(Message::SendUnsafeList(server.clone()))
    ]
    .spacing(8)
    .align_x(iced::Alignment::Center)
    .into()
}
