use std::path::PathBuf;

use chrono::format::SecondsFormat;
use chrono::{DateTime, Local, Utc};
use data::config::buffer::nickname::ShownStatus;
use data::dashboard::BufferAction;
use data::target::Target;
use data::{Config, Preview, Server, history, message};
use iced::widget::{
    self, button, column, container, operation, pick_list, row, rule, span,
    text, text_input,
};
use iced::{Color, Length, Size, Task, padding};

use super::context_menu::{self, Context};
use super::scroll_view;
use crate::widget::{
    Element, key_press, message_content, selectable_rich_text, selectable_text,
    tooltip,
};
use crate::{Theme, font, theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputSearchQueryFrom(String),
    InputSearchQueryTarget(String),
    InputSearchQueryText(String),
    InputSearchQueryTimestamp(String),
    SelectSearchQueryTimestampKind(SearchQueryTimestampKind),
    SendSearchQuery,
    Tab(bool),
}

pub enum Event {
    ContextMenu(context_menu::Event),
    OpenBuffer(Server, Target, BufferAction),
    GoToMessage(Server, Target, message::Hash),
    History(Task<history::manager::Message>),
    OpenUrl(String),
    ImagePreview(PathBuf, url::Url),
    ExpandCondensedMessage(DateTime<Utc>, message::Hash),
    ContractCondensedMessage(DateTime<Utc>, message::Hash),
    SendSearchQuery {
        server: Server,
        search_query: String,
    },
}

pub fn view<'a>(
    state: &'a SearchResults,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let timestamp_kinds = [
        SearchQueryTimestampKind::Before,
        SearchQueryTimestampKind::After,
    ];

    let valid_search_query = state.search_query.is_valid();

    let search_query = container(
        column![
            row![
                text_input("In..", &state.search_query.target)
                    .id(state.search_query.target_id.clone())
                    .style(move |theme, status| {
                        theme::text_input::hide_disabled(
                            theme::text_input::primary,
                            theme,
                            status,
                        )
                    })
                    .on_input(Message::InputSearchQueryTarget)
                    .on_submit_maybe(
                        valid_search_query.then_some(Message::SendSearchQuery)
                    )
            ],
            row![
                text_input("From..", &state.search_query.from)
                    .id(state.search_query.from_id.clone())
                    .style(move |theme, status| {
                        theme::text_input::hide_disabled(
                            theme::text_input::primary,
                            theme,
                            status,
                        )
                    })
                    .on_input(Message::InputSearchQueryFrom)
                    .on_submit_maybe(
                        valid_search_query.then_some(Message::SendSearchQuery)
                    )
            ],
            row![
                pick_list(
                    timestamp_kinds,
                    Some(state.search_query.timestamp_kind.clone()),
                    |search_timestamp_kind: SearchQueryTimestampKind| {
                        Message::SelectSearchQueryTimestampKind(
                            search_timestamp_kind.clone(),
                        )
                    }
                ),
                text_input("Timestamp..", &state.search_query.timestamp)
                    .id(state.search_query.timestamp_id.clone())
                    .style(move |theme, status| {
                        theme::text_input::hide_disabled(
                            theme::text_input::primary,
                            theme,
                            status,
                        )
                    })
                    .on_input(Message::InputSearchQueryTimestamp)
                    .on_submit_maybe(
                        valid_search_query.then_some(Message::SendSearchQuery)
                    )
            ]
            .spacing(8),
            row![
                text_input("Contains..", &state.search_query.text)
                    .id(state.search_query.text_id.clone())
                    .style(move |theme, status| {
                        theme::text_input::hide_disabled(
                            theme::text_input::primary,
                            theme,
                            status,
                        )
                    })
                    .on_input(Message::InputSearchQueryText)
                    .on_submit_maybe(
                        valid_search_query.then_some(Message::SendSearchQuery)
                    ),
                button(text("Search"))
                    .style(|theme, status| theme::button::secondary(
                        theme, status, false
                    ))
                    .on_press_maybe(
                        valid_search_query.then_some(Message::SendSearchQuery)
                    )
            ]
            .spacing(8),
            container(rule::horizontal(1)).width(Length::Fill),
        ]
        .spacing(8),
    )
    .padding(padding::horizontal(4))
    .width(Length::Fill);

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::SearchResults(&state.server),
            history,
            None,
            Option::<fn(&Preview, &message::Source) -> bool>::None,
            None,
            config,
            theme,
            move |message: &'a data::Message, _, _, _, _| match &message.target
            {
                message::Target::SearchResults {
                    target,
                    source: message::Source::User(user),
                } => {
                    let users = target
                        .as_ref()
                        .and_then(|target| target.as_channel())
                        .and_then(|channel| {
                            clients.get_channel_users(&state.server, channel)
                        });

                    let timestamp = config
                        .buffer
                        .format_timestamp(&message.server_time)
                        .map(|timestamp| {
                            context_menu::timestamp(
                                selectable_text(timestamp)
                                    .font_maybe(
                                        theme::font_style::timestamp(theme)
                                            .map(font::get),
                                    )
                                    .style(theme::selectable_text::timestamp),
                                &message.server_time,
                                config,
                                theme,
                            )
                            .map(scroll_view::Message::ContextMenu)
                        });

                    let target_text = target.as_ref().map(|target| {
                        selectable_rich_text::<_, _, (), _, _>(vec![
                            span(target.as_str())
                                .font_maybe(
                                    theme
                                        .styles()
                                        .buffer
                                        .url
                                        .font_style
                                        .map(font::get),
                                )
                                .color(theme.styles().buffer.url.color)
                                .link(message::Link::GoToMessage(
                                    state.server.clone(),
                                    target.clone(),
                                    message.hash,
                                )),
                            span(" "),
                        ])
                        .on_link(scroll_view::Message::Link)
                    });

                    let with_access_levels =
                        config.buffer.nickname.show_access_levels;
                    let truncate = config.buffer.nickname.truncate;

                    let current_user =
                        users.and_then(|users| users.resolve(user));
                    let is_user_offline =
                        match config.buffer.nickname.shown_status {
                            ShownStatus::Current => current_user.is_none(),
                            ShownStatus::Historical => false,
                        };

                    let text =
                        selectable_text(
                            config.buffer.nickname.brackets.format(
                                user.display(with_access_levels, truncate),
                            ),
                        )
                        .font_maybe(
                            theme::font_style::nickname(theme, is_user_offline)
                                .map(font::get),
                        )
                        .style(move |theme| {
                            theme::selectable_text::nickname(
                                theme,
                                config,
                                match config.buffer.nickname.shown_status {
                                    ShownStatus::Current => {
                                        current_user.unwrap_or(user)
                                    }
                                    ShownStatus::Historical => user,
                                },
                                is_user_offline,
                            )
                        });

                    let chantypes = clients.get_chantypes(&state.server);
                    let casemapping = clients.get_casemapping(&state.server);
                    let prefix = clients.get_prefix(&state.server);

                    let nick = tooltip(
                        context_menu::user(
                            text,
                            &state.server,
                            prefix,
                            target
                                .as_ref()
                                .and_then(|target| target.as_channel()),
                            user,
                            current_user,
                            None,
                            config,
                            theme,
                            &config.buffer.nickname.click,
                        )
                        .map(scroll_view::Message::ContextMenu),
                        // We show the full nickname in the tooltip if truncation is enabled.
                        truncate.map(|_| user.as_str()),
                        tooltip::Position::Bottom,
                        theme,
                    );

                    let text = message_content::with_context(
                        &message.content,
                        &state.server,
                        chantypes,
                        casemapping,
                        theme,
                        scroll_view::Message::Link,
                        None,
                        theme::selectable_text::default,
                        theme::font_style::primary,
                        Option::<fn(Color) -> Color>::None,
                        move |link| match link {
                            message::Link::User(_, _) => {
                                context_menu::Entry::user_list(
                                    true,
                                    current_user,
                                    None,
                                    config.file_transfer.enabled,
                                )
                            }
                            message::Link::Url(_) => {
                                context_menu::Entry::url_list()
                            }
                            _ => vec![],
                        },
                        move |link, entry, length| {
                            let context = if let Some(user) = link.user() {
                                Some(Context::User {
                                    server: &state.server,
                                    prefix,
                                    channel: target
                                        .as_ref()
                                        .and_then(|target| target.as_channel()),
                                    user,
                                    current_user,
                                })
                            } else {
                                link.url().map(Context::Url)
                            };

                            entry
                                .view(context, length, config, theme)
                                .map(scroll_view::Message::ContextMenu)
                        },
                        config,
                    );

                    Some(
                        container(row![
                            timestamp,
                            selectable_text(" "),
                            target_text,
                            nick,
                            selectable_text(" "),
                            text,
                        ])
                        .into(),
                    )
                }
                message::Target::SearchResults {
                    target,
                    source: message::Source::Action(_),
                } => {
                    let timestamp = config
                        .buffer
                        .format_timestamp(&message.server_time)
                        .map(|timestamp| {
                            selectable_text(timestamp)
                                .font_maybe(
                                    theme::font_style::timestamp(theme)
                                        .map(font::get),
                                )
                                .style(theme::selectable_text::timestamp)
                        });

                    let target_text = target.as_ref().map(|target| {
                        selectable_rich_text::<_, _, (), _, _>(vec![
                            span(target.as_str())
                                .color(theme.styles().buffer.url.color)
                                .link(message::Link::GoToMessage(
                                    state.server.clone(),
                                    target.clone(),
                                    message.hash,
                                )),
                            span(" "),
                        ])
                        .on_link(scroll_view::Message::Link)
                    });

                    let chantypes = clients.get_chantypes(&state.server);
                    let casemapping = clients.get_casemapping(&state.server);

                    let text = message_content(
                        &message.content,
                        &state.server,
                        chantypes,
                        casemapping,
                        theme,
                        scroll_view::Message::Link,
                        None,
                        theme::selectable_text::action,
                        theme::font_style::action,
                        Option::<fn(Color) -> Color>::None,
                        config,
                    );

                    Some(
                        container(row![
                            timestamp,
                            selectable_text(" "),
                            target_text,
                            text
                        ])
                        .into(),
                    )
                }
                message::Target::SearchResults {
                    target: None,
                    source: message::Source::Server(server),
                } => {
                    let timestamp = config
                        .buffer
                        .format_timestamp(&message.server_time)
                        .map(|timestamp| {
                            context_menu::timestamp(
                                selectable_text(timestamp)
                                    .font_maybe(
                                        theme::font_style::timestamp(theme)
                                            .map(font::get),
                                    )
                                    .style(theme::selectable_text::timestamp),
                                &message.server_time,
                                config,
                                theme,
                            )
                            .map(scroll_view::Message::ContextMenu)
                        });

                    let chantypes = clients.get_chantypes(&state.server);
                    let casemapping = clients.get_casemapping(&state.server);

                    let text = message_content(
                        &message.content,
                        &state.server,
                        chantypes,
                        casemapping,
                        theme,
                        scroll_view::Message::Link,
                        None,
                        move |theme| {
                            theme::selectable_text::server(
                                theme,
                                server.as_ref(),
                            )
                        },
                        move |theme| {
                            theme::font_style::server(theme, server.as_ref())
                        },
                        Option::<fn(Color) -> Color>::None,
                        config,
                    );

                    Some(
                        container(row![timestamp, selectable_text(" "), text])
                            .into(),
                    )
                }
                _ => None,
            },
        )
        .map(Message::ScrollView),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding([0, 6]);

    let content = column![
        key_press(
            key_press(
                search_query,
                key_press::Key::Named(key_press::Named::Tab),
                key_press::Modifiers::SHIFT,
                Message::Tab(true),
            ),
            key_press::Key::Named(key_press::Named::Tab),
            key_press::Modifiers::default(),
            Message::Tab(false),
        ),
        messages
    ]
    .padding([8, 2]);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[derive(Debug, Clone, Default, PartialEq, strum::Display)]
pub enum SearchQueryTimestampKind {
    #[default]
    After,
    Before,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub from: String,
    from_id: widget::Id,
    pub timestamp: String,
    timestamp_id: widget::Id,
    timestamp_kind: SearchQueryTimestampKind,
    pub target: String,
    target_id: widget::Id,
    pub text: String,
    text_id: widget::Id,
}

impl SearchQuery {
    pub fn new(target: Option<Target>, text: Option<String>) -> Self {
        Self {
            from: String::new(),
            from_id: widget::Id::unique(),
            timestamp: String::new(),
            timestamp_id: widget::Id::unique(),
            timestamp_kind: SearchQueryTimestampKind::default(),
            target: target.map(|target| target.to_string()).unwrap_or_default(),
            target_id: widget::Id::unique(),
            text: text.unwrap_or_default(),
            text_id: widget::Id::unique(),
        }
    }

    pub fn is_valid(&self) -> bool {
        if self.target.is_empty() {
            return false;
        }

        if !self.timestamp.is_empty() {
            self.timestamp.as_str().parse::<DateTime<Local>>().is_ok()
        } else {
            !self.from.is_empty() || !self.text.is_empty()
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResults {
    pub server: data::server::Server,
    pub search_query: SearchQuery,
    pub scroll_view: scroll_view::State,
}

impl SearchResults {
    pub fn new(
        server: data::server::Server,
        target: Option<Target>,
        text: Option<String>,
        pane_size: Size,
        config: &Config,
    ) -> Self {
        Self {
            server,
            search_query: SearchQuery::new(target, text),
            scroll_view: scroll_view::State::new(pane_size, config),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        history: &history::Manager,
        clients: &data::client::Map,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(
                    message,
                    false,
                    scroll_view::Kind::SearchResults(&self.server),
                    history,
                    clients,
                    config,
                );

                let event = event.and_then(|event| match event {
                    scroll_view::Event::ContextMenu(event) => {
                        Some(Event::ContextMenu(event))
                    }
                    scroll_view::Event::OpenBuffer(
                        server,
                        target,
                        buffer_action,
                    ) => Some(Event::OpenBuffer(server, target, buffer_action)),
                    scroll_view::Event::GoToMessage(
                        server,
                        target,
                        message,
                    ) => Some(Event::GoToMessage(server, target, message)),
                    scroll_view::Event::RequestOlderChatHistory => None,
                    scroll_view::Event::PreviewChanged => None,
                    scroll_view::Event::HidePreview(..) => None,
                    scroll_view::Event::MarkAsRead => None,
                    scroll_view::Event::OpenUrl(url) => {
                        Some(Event::OpenUrl(url))
                    }
                    scroll_view::Event::ImagePreview(path, url) => {
                        Some(Event::ImagePreview(path, url))
                    }
                    scroll_view::Event::ExpandCondensedMessage(
                        server_time,
                        hash,
                    ) => Some(Event::ExpandCondensedMessage(server_time, hash)),
                    scroll_view::Event::ContractCondensedMessage(
                        server_time,
                        hash,
                    ) => {
                        Some(Event::ContractCondensedMessage(server_time, hash))
                    }
                });

                (command.map(Message::ScrollView), event)
            }
            Message::InputSearchQueryFrom(from) => {
                self.search_query.from = from;

                (Task::none(), None)
            }
            Message::InputSearchQueryTarget(target) => {
                self.search_query.target = target;

                (Task::none(), None)
            }
            Message::InputSearchQueryText(text) => {
                self.search_query.text = text;

                (Task::none(), None)
            }
            Message::InputSearchQueryTimestamp(timestamp) => {
                self.search_query.timestamp = timestamp;

                (Task::none(), None)
            }
            Message::SelectSearchQueryTimestampKind(timestamp_kind) => {
                self.search_query.timestamp_kind = timestamp_kind;

                (Task::none(), None)
            }
            Message::SendSearchQuery => {
                let mut search_query =
                    format!("in={}", self.search_query.target);

                if !self.search_query.from.is_empty() {
                    search_query.push_str(";from=");
                    search_query.push_str(&self.search_query.from);
                }

                if !self.search_query.timestamp.is_empty()
                    && let Ok(timestamp) =
                        self.search_query.timestamp.parse::<DateTime<Local>>()
                {
                    match self.search_query.timestamp_kind {
                        SearchQueryTimestampKind::After => {
                            search_query.push_str(";after=");
                        }
                        SearchQueryTimestampKind::Before => {
                            search_query.push_str(";before=");
                        }
                    }
                    search_query.push_str(
                        &timestamp
                            .to_utc()
                            .to_rfc3339_opts(SecondsFormat::Millis, true),
                    );
                }

                if !self.search_query.text.is_empty() {
                    search_query.push_str(";text=");
                    search_query.push_str(&self.search_query.text);
                }

                (
                    Task::none(),
                    Some(Event::SendSearchQuery {
                        server: self.server.clone(),
                        search_query,
                    }),
                )
            }
            Message::Tab(shift) => {
                let search_query_from_id = self.search_query.from_id.clone();
                let search_query_target_id =
                    self.search_query.target_id.clone();
                let search_query_timestamp_id =
                    self.search_query.timestamp_id.clone();
                let search_query_text_id = self.search_query.text_id.clone();

                let task = Task::batch(vec![
                    operation::is_focused(search_query_from_id.clone()).map(
                        move |is_focused| {
                            (is_focused, search_query_from_id.clone())
                        },
                    ),
                    operation::is_focused(search_query_target_id.clone()).map(
                        move |is_focused| {
                            (is_focused, search_query_target_id.clone())
                        },
                    ),
                    operation::is_focused(search_query_timestamp_id.clone())
                        .map(move |is_focused| {
                            (is_focused, search_query_timestamp_id.clone())
                        }),
                    operation::is_focused(search_query_text_id.clone()).map(
                        move |is_focused| {
                            (is_focused, search_query_text_id.clone())
                        },
                    ),
                ])
                .collect();

                let search_query_from_id = self.search_query.from_id.clone();
                let search_query_target_id =
                    self.search_query.target_id.clone();
                let search_query_timestamp_id =
                    self.search_query.timestamp_id.clone();
                let search_query_text_id = self.search_query.text_id.clone();

                let task = task.then(move |are_focused| {
                    if let Some(id) = are_focused
                        .into_iter()
                        .find_map(|(is_focused, id)| is_focused.then_some(id))
                    {
                        if id == search_query_target_id {
                            if shift {
                                operation::focus(search_query_text_id.clone())
                            } else {
                                operation::focus(search_query_from_id.clone())
                            }
                        } else if id == search_query_from_id {
                            if shift {
                                operation::focus(search_query_target_id.clone())
                            } else {
                                operation::focus(
                                    search_query_timestamp_id.clone(),
                                )
                            }
                        } else if id == search_query_timestamp_id {
                            if shift {
                                operation::focus(search_query_from_id.clone())
                            } else {
                                operation::focus(search_query_text_id.clone())
                            }
                        } else if id == search_query_text_id {
                            if shift {
                                operation::focus(
                                    search_query_timestamp_id.clone(),
                                )
                            } else {
                                operation::focus(search_query_target_id.clone())
                            }
                        } else {
                            Task::none()
                        }
                    } else {
                        Task::none()
                    }
                });

                (task, None)
            }
        }
    }

    pub fn focus(&self) -> Task<Message> {
        let search_query_from_id = self.search_query.from_id.clone();
        let search_query_target_id = self.search_query.target_id.clone();
        let search_query_timestamp_id = self.search_query.timestamp_id.clone();
        let search_query_text_id = self.search_query.text_id.clone();

        Task::batch(vec![
            operation::is_focused(search_query_from_id.clone()),
            operation::is_focused(search_query_target_id.clone()),
            operation::is_focused(search_query_timestamp_id.clone()),
            operation::is_focused(search_query_text_id.clone()),
        ])
        .collect()
        .then(move |are_focused| {
            if are_focused.into_iter().any(|is_focused| is_focused) {
                Task::none()
            } else {
                operation::focus(search_query_text_id.clone())
            }
        })
    }

    pub fn update_search_query(
        &mut self,
        target: Option<String>,
        text: Option<String>,
    ) {
        if let Some(target) = target {
            self.search_query.target = target;
        }

        if let Some(text) = text {
            self.search_query.text = text;
        }
    }
}
