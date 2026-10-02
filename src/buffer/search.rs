use std::time::Duration;

use data::client::{self, ClientsContext};
use data::config::actions::NicknameClickAction;
use data::dashboard::BufferAction;
use data::history::filter::FilterChain;
use data::history::{self, search, storage};
use data::target::{self, Target};
use data::user::Nick;
use data::{Config, Server, buffer, message, preview};
use iced::widget::{
    self, button, center, column, container, operation, row, rule, scrollable,
    text, text_input,
};
use iced::{ContentFit, Length, Task, padding};
use tokio::time;

use super::{context_menu, message_feed, scroll_view};
use crate::widget::Element;
use crate::{Theme, font, icon, theme};

const DEBOUNCE: Duration = Duration::from_millis(200);

#[derive(Debug, Clone)]
pub enum Message {
    Input(String),
    Debounced(u64),
    Refresh,
    Results(u64, bool, Result<search::Page, search::Error>),
    LoadMore,
    Feed(scroll_view::Message),
}

pub enum Event {
    ContextMenu(context_menu::Event),
    OpenBuffer(Server, Target, BufferAction),
    GoToMessage(buffer::Upstream, message::MessageLink, BufferAction),
    OpenUrl(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Idle,
    Stale,
    Loading,
    Loaded,
    Failed(String),
}

#[derive(Debug, Clone)]
struct Hit {
    buffer: buffer::Upstream,
    message: data::Message,
}

#[derive(Debug, Clone)]
pub struct Search {
    pub query: String,
    input_id: widget::Id,
    generation: u64,
    status: Status,
    hits: Vec<Hit>,
    next: Option<search::Cursor>,
}

impl Search {
    pub fn new(query: Option<String>) -> Self {
        let mut search = Self {
            query: String::new(),
            input_id: widget::Id::unique(),
            generation: 0,
            status: Status::Idle,
            hits: vec![],
            next: None,
        };
        search.set_query(query.unwrap_or_default());
        search
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.generation += 1;
        self.hits.clear();
        self.next = None;
        self.status = if search::Query::parse(&self.query).is_runnable() {
            Status::Stale
        } else {
            Status::Idle
        };
    }

    pub fn has_more(&self) -> bool {
        self.next.is_some()
    }

    pub fn result_count(&self) -> Option<usize> {
        matches!(self.status, Status::Loaded).then_some(self.hits.len())
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &client::Map,
        storage: &storage::Manager,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Input(query) => {
                self.query = query;
                self.generation += 1;
                self.next = None;

                if search::Query::parse(&self.query).is_runnable() {
                    self.status = Status::Stale;
                    let generation = self.generation;

                    (
                        Task::perform(time::sleep(DEBOUNCE), move |()| {
                            Message::Debounced(generation)
                        }),
                        None,
                    )
                } else {
                    self.hits.clear();
                    self.status = Status::Idle;

                    (Task::none(), None)
                }
            }
            Message::Debounced(generation) => {
                if generation == self.generation && self.status == Status::Stale
                {
                    (self.run(storage, None), None)
                } else {
                    (Task::none(), None)
                }
            }
            Message::Refresh => {
                if self.status == Status::Stale {
                    (self.run(storage, None), None)
                } else {
                    (Task::none(), None)
                }
            }
            Message::LoadMore => {
                if self.status == Status::Loaded
                    && let Some(next) = self.next
                {
                    (self.run(storage, Some(next)), None)
                } else {
                    (Task::none(), None)
                }
            }
            Message::Results(generation, append, result) => {
                if generation != self.generation {
                    return (Task::none(), None);
                }

                match result {
                    Ok(page) => {
                        if !append {
                            self.hits.clear();
                        }
                        self.hits.extend(
                            page.hits
                                .into_iter()
                                .filter_map(|hit| resolve(hit, clients)),
                        );
                        self.next = page.next;
                        self.status = Status::Loaded;
                    }
                    Err(error) => {
                        self.hits.clear();
                        self.next = None;
                        self.status = Status::Failed(error.to_string());
                    }
                }

                (Task::none(), None)
            }
            Message::Feed(message) => {
                (Task::none(), feed_event(message, config))
            }
        }
    }

    fn run(
        &mut self,
        storage: &storage::Manager,
        before: Option<search::Cursor>,
    ) -> Task<Message> {
        self.status = Status::Loading;

        let generation = self.generation;
        let append = before.is_some();

        Task::perform(
            storage.search(search::Query::parse(&self.query), before),
            move |result| Message::Results(generation, append, result),
        )
    }

    pub fn focus(&self) -> Task<Message> {
        let input_id = self.input_id.clone();

        let focus =
            operation::is_focused(input_id.clone()).then(move |is_focused| {
                if is_focused {
                    Task::none()
                } else {
                    operation::focus(input_id.clone())
                }
            });

        if self.status == Status::Stale {
            focus.chain(Task::done(Message::Refresh))
        } else {
            focus
        }
    }
}

fn resolve(hit: search::Hit, clients: &client::Map) -> Option<Hit> {
    let server = clients
        .servers()
        .find(|server| format!("{server:b}") == hit.server)?
        .clone();

    let buffer = match hit.buffer {
        search::Buffer::Server => buffer::Upstream::Server(server),
        search::Buffer::Channel(channel) => {
            let channel = target::Channel::from_str(
                &channel,
                clients.get_server_chantypes_or_default(&server),
                clients.get_server_casemapping_or_default(&server),
            );
            buffer::Upstream::Channel(server, channel)
        }
        search::Buffer::Query(query) => {
            let query = target::Query::from(Nick::from_str(
                &query,
                clients.get_server_casemapping_or_default(&server),
            ));
            buffer::Upstream::Query(server, query)
        }
    };

    Some(Hit {
        buffer,
        message: hit.message,
    })
}

fn feed_event(message: scroll_view::Message, config: &Config) -> Option<Event> {
    match message {
        scroll_view::Message::ContextMenu(message) => {
            context_menu::update(message).map(Event::ContextMenu)
        }
        scroll_view::Message::Link(link) => match link {
            message::Link::GoToMessage(buffer, message, buffer_action) => {
                Some(Event::GoToMessage(
                    buffer,
                    message,
                    buffer_action.unwrap_or_default(),
                ))
            }
            message::Link::Channel(server, channel, buffer_action) => {
                buffer_action.map(|buffer_action| {
                    Event::OpenBuffer(
                        server,
                        Target::Channel(channel),
                        buffer_action,
                    )
                })
            }
            message::Link::Url(url) => Some(Event::OpenUrl(url)),
            message::Link::User(server, user) => {
                match config.actions.buffer.click_username {
                    NicknameClickAction::OpenQuery(buffer_action) => {
                        Some(Event::OpenBuffer(
                            server,
                            Target::Query(target::Query::from(user)),
                            buffer_action,
                        ))
                    }
                    NicknameClickAction::InsertNickname
                    | NicknameClickAction::Noop => None,
                }
            }
            message::Link::ExpandMessage(..)
            | message::Link::ContractMessage(..) => None,
        },
        _ => None,
    }
}

pub fn view<'a>(
    state: &'a Search,
    clients: &'a client::Map,
    previews: &'a preview::Collection,
    filter_chain: FilterChain<'_>,
    config: &'a Config,
    theme: &'a Theme,
    channels_context: &'a dyn context_menu::ChannelsContext,
) -> Element<'a, Message> {
    let header = container(
        column![
            row![
                text_input("Search messages...", &state.query)
                    .id(state.input_id.clone())
                    .style(move |theme, status| {
                        if matches!(status, text_input::Status::Disabled) {
                            theme::text_input::primary(
                                theme,
                                text_input::Status::Active,
                            )
                        } else {
                            theme::text_input::primary(theme, status)
                        }
                    })
                    .on_input(Message::Input),
            ]
            .spacing(8)
            .padding(padding::top(8)),
            container(rule::horizontal(1)).width(Length::Fill)
        ]
        .spacing(8),
    )
    .padding(padding::horizontal(4))
    .width(Length::Fill);

    let rows = state
        .hits
        .iter()
        .filter(|hit| {
            !filter_chain.filter_message_of_kind(
                &hit.message,
                &history::Kind::from(hit.buffer.clone()),
            )
        })
        .filter_map(|hit| {
            message_feed::message_row(
                &hit.message,
                hit.buffer.clone(),
                hit.buffer.as_server(),
                hit.buffer.as_channel(),
                message::MessageLink::Message(hit.message.history_id),
                clients,
                previews,
                config,
                theme,
                channels_context,
            )
        })
        .map(|row| row.map(Message::Feed))
        .collect::<Vec<_>>();

    let body: Element<'a, Message> = if rows.is_empty() {
        let reason = match &state.status {
            Status::Idle => None,
            Status::Stale | Status::Loading => Some("...".to_string()),
            Status::Loaded => Some("No messages found".to_string()),
            Status::Failed(error) => Some(error.clone()),
        };

        placeholder(reason, theme)
    } else {
        let load_more = (state.has_more() && state.status == Status::Loaded)
            .then(|| {
                container(
                    button(text("Load more"))
                        .style(|theme, status| {
                            theme::button::secondary(theme, status, false)
                        })
                        .on_press(Message::LoadMore),
                )
                .center_x(Length::Fill)
                .padding(8)
            });

        scrollable(
            column(rows)
                .push(load_more)
                .spacing(config.buffer.line_spacing)
                .padding(8),
        )
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::default()
                .width(config.pane.scrollbar.width)
                .scroller_width(config.pane.scrollbar.scroller_width),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    };

    let content = column![header, body].spacing(1).padding([2, 2]);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn placeholder<'a>(
    reason: Option<String>,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let content = match reason {
        Some(reason) => column![
            text(reason)
                .style(theme::text::secondary)
                .font_maybe(theme::font_style::secondary(theme).map(font::get))
        ],
        None => column![
            icon::search()
                .width(Length::Shrink)
                .content_fit(ContentFit::Contain)
                .height(theme::TEXT_SIZE + 3.0)
                .style(theme::text::secondary),
            text("Search messages")
                .style(theme::text::secondary)
                .font_maybe(theme::font_style::secondary(theme).map(font::get)),
        ]
        .spacing(8)
        .align_x(iced::Alignment::Center),
    };

    center(content).into()
}
