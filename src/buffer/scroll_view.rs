use chrono::{DateTime, Utc};
use data::isupport::ChatHistoryState;
use data::message::{self, Limit};
use data::server::Server;
use data::user::Nick;
use data::{history, Config};
use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, row, scrollable, text, Scrollable,
};
use iced::{padding, Length, Task};

use self::keyed::keyed;
use super::user_context;
use crate::widget::{Element, MESSAGE_MARKER_TEXT};
use crate::{font, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Scrolled {
        count: usize,
        remaining: bool,
        oldest: DateTime<Utc>,
        status: Status,
        viewport: scrollable::Viewport,
    },
    UserContext(user_context::Message),
    Link(message::Link),
    ScrollTo(keyed::Bounds),
    RequestOlderChatHistory,
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
    OpenChannel(String),
    GoToMessage(Server, String, message::Hash),
    RequestOlderChatHistory,
}

#[derive(Debug, Clone, Copy)]
pub enum Kind<'a> {
    Server(&'a Server),
    Channel(&'a Server, &'a str),
    Query(&'a Server, &'a Nick),
    Logs,
    Highlights,
}

impl From<Kind<'_>> for history::Kind {
    fn from(value: Kind<'_>) -> Self {
        match value {
            Kind::Server(server) => history::Kind::Server(server.clone()),
            Kind::Channel(server, channel) => {
                history::Kind::Channel(server.clone(), channel.to_string())
            }
            Kind::Query(server, nick) => history::Kind::Query(server.clone(), nick.clone()),
            Kind::Logs => history::Kind::Logs,
            Kind::Highlights => history::Kind::Highlights,
        }
    }
}

pub fn view<'a>(
    state: &State,
    kind: Kind,
    history: &'a history::Manager,
    chathistory_state: Option<ChatHistoryState>,
    config: &'a Config,
    format: impl Fn(&'a data::Message, Option<f32>, Option<f32>) -> Option<Element<'a, Message>> + 'a,
) -> Element<'a, Message> {
    let Some(history::View {
        total,
        old_messages,
        new_messages,
        max_nick_chars,
        max_prefix_chars,
    }) = history.get_messages(&kind.into(), Some(state.limit), &config.buffer)
    else {
        return column![].into();
    };

    let top_row = if let Some(chathistory_state) = chathistory_state {
        let (content, message) = match chathistory_state {
            ChatHistoryState::Exhausted => ("No Older Chat History Messages Available", None),
            ChatHistoryState::PendingRequest => ("...", None),
            ChatHistoryState::Ready => (
                "Request Older Chat History Messages",
                Some(Message::RequestOlderChatHistory),
            ),
        };

        let font_size = config.font.size.map(f32::from).unwrap_or(theme::TEXT_SIZE) - 1.0;

        let top_row_button = button(text(content).size(font_size))
            .padding([3, 5])
            .style(|theme, status| theme::button::primary(theme, status, false))
            .on_press_maybe(message);

        Some(
            row![horizontal_space(), top_row_button, horizontal_space()]
                .padding(padding::top(2).bottom(6))
                .width(Length::Fill)
                .align_y(iced::Alignment::Center),
        )
    } else {
        None
    };

    let count = old_messages.len() + new_messages.len();
    let remaining = count < total;
    let oldest = old_messages
        .iter()
        .chain(&new_messages)
        .next()
        .map(|message| message.server_time)
        .unwrap_or_else(Utc::now);
    let status = state.status;

    let max_nick_width = max_nick_chars.map(|len| {
        font::width_from_chars(
            usize::max(len, MESSAGE_MARKER_TEXT.chars().count()),
            &config.font,
        )
    });

    let max_prefix_width = max_prefix_chars.map(|len| font::width_from_chars(len, &config.font));

    let old = old_messages
        .into_iter()
        .filter_map(|message| {
            format(message, max_nick_width, max_prefix_width)
                .map(|element| keyed(keyed::Key::message(message), element))
        })
        .collect::<Vec<_>>();
    let new = new_messages
        .into_iter()
        .filter_map(|message| {
            format(message, max_nick_width, max_prefix_width)
                .map(|element| keyed(keyed::Key::message(message), element))
        })
        .collect::<Vec<_>>();

    let show_divider =
        !new.is_empty() || matches!(status, Status::Idle(Anchor::Bottom) | Status::ScrollTo);

    let divider = if show_divider {
        let font_size = config.font.size.map(f32::from).unwrap_or(theme::TEXT_SIZE) - 1.0;

        row![
            container(horizontal_rule(1))
                .width(Length::Fill)
                .padding(padding::right(6)),
            text("backlog")
                .size(font_size)
                .style(theme::text::secondary),
            container(horizontal_rule(1))
                .width(Length::Fill)
                .padding(padding::left(6))
        ]
        .padding(2)
        .align_y(iced::Alignment::Center)
    } else {
        row![]
    };

    let content = column![]
        .push_maybe(top_row)
        .push(column(old))
        .push(keyed(keyed::Key::Divider, divider))
        .push(column(new));

    Scrollable::new(container(content).width(Length::Fill).padding([0, 8]))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::default()
                .anchor(status.alignment())
                .width(5)
                .scroller_width(5),
        ))
        .on_scroll(move |viewport| Message::Scrolled {
            count,
            remaining,
            oldest,
            status,
            viewport,
        })
        .id(state.scrollable.clone())
        .into()
}

#[derive(Debug, Clone)]
pub struct State {
    pub scrollable: scrollable::Id,
    limit: Limit,
    status: Status,
    pending_scroll_to: Option<message::Hash>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scrollable: scrollable::Id::unique(),
            limit: Limit::bottom(),
            status: Status::default(),
            pending_scroll_to: None,
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(
        &mut self,
        message: Message,
        infinite_scroll: bool,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Scrolled {
                count,
                remaining,
                oldest,
                status: old_status,
                viewport,
            } => {
                let relative_offset = viewport.relative_offset().y;

                match old_status {
                    Status::ScrollTo => {
                        return (Task::none(), None);
                    }
                    Status::Loading(anchor) => {
                        self.status = Status::Unlocked(anchor);

                        if matches!(anchor, Anchor::Bottom) {
                            self.limit = Limit::Since(oldest);
                        }
                        // Top anchor can get stuck in loading state at
                        // end of scrollable.
                        else if old_status.is_end(relative_offset) {
                            if remaining {
                                self.status = Status::Loading(Anchor::Top);
                                self.limit = Limit::Top(count + Limit::DEFAULT_STEP);
                            } else {
                                self.status = Status::Idle(Anchor::Bottom);
                                self.limit = Limit::bottom();
                            }
                        }
                    }
                    _ if old_status.is_end(relative_offset) && remaining => {
                        match old_status.anchor() {
                            Anchor::Top => {
                                self.status = Status::Loading(Anchor::Top);
                                self.limit = Limit::Top(count + Limit::DEFAULT_STEP);
                            }
                            Anchor::Bottom => {
                                self.status = Status::Loading(Anchor::Bottom);
                                self.limit = Limit::Bottom(count + Limit::DEFAULT_STEP);
                            }
                        }
                    }
                    _ if old_status.is_bottom(relative_offset) => {
                        self.status = Status::Idle(Anchor::Bottom);
                        self.limit = Limit::bottom();
                    }
                    _ if old_status.is_top(relative_offset) => {
                        self.status = Status::Idle(Anchor::Top);
                        self.limit = Limit::top();
                    }
                    Status::Idle(anchor) if !old_status.is_start(relative_offset) => {
                        self.status = Status::Unlocked(anchor);

                        if matches!(anchor, Anchor::Bottom) {
                            self.limit = Limit::Since(oldest);
                        }
                    }
                    Status::Unlocked(_) | Status::Idle(_) => {}
                }

                if let Some(new_offset) = self.status.new_offset(old_status, viewport) {
                    return (
                        scrollable::scroll_to(self.scrollable.clone(), new_offset),
                        None,
                    );
                } else if infinite_scroll && self.status.is_top(relative_offset) {
                    return (Task::none(), Some(Event::RequestOlderChatHistory));
                }
            }
            Message::UserContext(message) => {
                return (
                    Task::none(),
                    user_context::update(message).map(Event::UserContext),
                );
            }
            Message::Link(message::Link::Channel(channel)) => {
                return (Task::none(), Some(Event::OpenChannel(channel)))
            }
            Message::Link(message::Link::Url(url)) => {
                let _ = open::that_detached(url);
            }
            Message::Link(message::Link::User(user)) => {
                return (
                    Task::none(),
                    Some(Event::UserContext(user_context::Event::SingleClick(
                        user.nickname().to_owned(),
                    ))),
                )
            }
            Message::Link(message::Link::GoToMessage(server, channel, message)) => {
                return (
                    Task::none(),
                    Some(Event::GoToMessage(server, channel, message)),
                )
            }
            Message::ScrollTo(keyed::Bounds {
                scrollable_bounds,
                hit_bounds,
                prev_bounds,
            }) => {
                let total_offset =
                    scrollable_bounds.content.height - scrollable_bounds.viewport.height;

                let absolute = hit_bounds.y - scrollable_bounds.content.y;
                let relative = (absolute / total_offset).min(1.0);

                self.status = Status::Idle(Anchor::Bottom);

                // Offsets are given relative to top,
                // and we must scroll to offsets relative to
                // the bottom
                let offset = if relative == 1.0 {
                    0.0
                } else {
                    // If a prev element exists, put scrollable halfway over prev
                    // element so it's obvious user can scroll up
                    if let Some(bounds) = prev_bounds {
                        let absolute =
                            (bounds.y - scrollable_bounds.content.y) + bounds.height / 2.0;

                        total_offset - absolute
                    } else {
                        total_offset - absolute
                    }
                };

                return (
                    scrollable::scroll_to(
                        self.scrollable.clone(),
                        scrollable::AbsoluteOffset { x: 0.0, y: offset },
                    ),
                    None,
                );
            }
            Message::RequestOlderChatHistory => {
                return (Task::none(), Some(Event::RequestOlderChatHistory))
            }
        }

        (Task::none(), None)
    }

    pub fn scroll_to_start(&mut self) -> Task<Message> {
        self.status = Status::Idle(Anchor::Top);
        self.limit = Limit::top();
        scrollable::scroll_to(
            self.scrollable.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
        )
    }

    pub fn scroll_to_end(&mut self) -> Task<Message> {
        self.status = Status::Idle(Anchor::Bottom);
        self.limit = Limit::bottom();
        scrollable::scroll_to(
            self.scrollable.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
        )
    }

    pub fn is_scrolled_to_bottom(&self) -> bool {
        matches!(self.status, Status::Idle(Anchor::Bottom))
    }

    pub fn scroll_to_message(
        &mut self,
        message: message::Hash,
        kind: Kind,
        history: &history::Manager,
        config: &Config,
    ) -> Task<Message> {
        let Some(history::View {
            total,
            old_messages,
            new_messages,
            ..
        }) = history.get_messages(&kind.into(), None, &config.buffer)
        else {
            // We're still loading history, which will trigger
            // scroll_to_backlog after loading. If this is set,
            // we will scroll_to_message
            self.pending_scroll_to = Some(message);

            return Task::none();
        };

        let Some(pos) = old_messages
            .iter()
            .chain(&new_messages)
            .position(|m| m.hash == message)
        else {
            return Task::none();
        };

        // Get all messages from bottom until 1 before message
        let offset = total - pos + 1;

        self.limit = Limit::Bottom(offset.max(Limit::DEFAULT_COUNT));
        self.status = Status::ScrollTo;

        keyed::find_bounds(self.scrollable.clone(), keyed::Key::Message(message))
            .map(Message::ScrollTo)
    }

    pub fn scroll_to_backlog(
        &mut self,
        kind: Kind,
        history: &history::Manager,
        config: &Config,
    ) -> Task<Message> {
        if let Some(message) = self.pending_scroll_to.take() {
            return self.scroll_to_message(message, kind, history, config);
        }

        if history.read_marker(&kind.into()).is_none() {
            return Task::none();
        }

        let Some(history::View {
            total,
            old_messages,
            ..
        }) = history.get_messages(&kind.into(), None, &config.buffer)
        else {
            return Task::none();
        };

        // Get all messages from bottom until 1 before backlog
        let offset = total - old_messages.len() + 1;

        self.limit = Limit::Bottom(offset.max(Limit::DEFAULT_COUNT));
        self.status = Status::ScrollTo;

        keyed::find_bounds(self.scrollable.clone(), keyed::Key::Divider).map(Message::ScrollTo)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Idle(Anchor),
    Unlocked(Anchor),
    Loading(Anchor),
    ScrollTo,
}

#[derive(Debug, Clone, Copy)]
pub enum Anchor {
    Top,
    Bottom,
}

impl Status {
    fn anchor(self) -> Anchor {
        match self {
            Status::Idle(anchor) => anchor,
            Status::Unlocked(anchor) => anchor,
            Status::Loading(anchor) => anchor,
            Status::ScrollTo => Anchor::Bottom,
        }
    }

    fn alignment(self) -> scrollable::Anchor {
        match self {
            Status::Idle(anchor) => match anchor {
                Anchor::Top => scrollable::Anchor::Start,
                Anchor::Bottom => scrollable::Anchor::End,
            },
            Status::Unlocked(_) => scrollable::Anchor::Start,
            Status::Loading(anchor) => match anchor {
                Anchor::Top => scrollable::Anchor::Start,
                Anchor::Bottom => scrollable::Anchor::End,
            },
            Status::ScrollTo => scrollable::Anchor::Start,
        }
    }

    fn is_end(self, relative_offset: f32) -> bool {
        match self.anchor() {
            Anchor::Top => self.is_bottom(relative_offset),
            Anchor::Bottom => self.is_top(relative_offset),
        }
    }

    fn is_start(self, relative_offset: f32) -> bool {
        match self.anchor() {
            Anchor::Top => self.is_top(relative_offset),
            Anchor::Bottom => self.is_bottom(relative_offset),
        }
    }

    fn is_top(self, relative_offset: f32) -> bool {
        match self.alignment() {
            scrollable::Anchor::Start => relative_offset == 0.0,
            scrollable::Anchor::End => relative_offset == 1.0,
        }
    }

    fn is_bottom(self, relative_offset: f32) -> bool {
        match self.alignment() {
            scrollable::Anchor::Start => relative_offset == 1.0,
            scrollable::Anchor::End => relative_offset == 0.0,
        }
    }

    fn new_offset(
        self,
        other: Self,
        viewport: scrollable::Viewport,
    ) -> Option<scrollable::AbsoluteOffset> {
        let old = self.alignment();
        let new = other.alignment();

        if old != new {
            let offset = viewport.absolute_offset();
            let reversed_offset = viewport.absolute_offset_reversed();

            Some(scrollable::AbsoluteOffset {
                x: offset.x,
                y: reversed_offset.y,
            })
        } else {
            None
        }
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::Idle(Anchor::Bottom)
    }
}

mod keyed {
    use data::message;
    use iced::advanced::widget::{self, Operation};
    use iced::widget::scrollable;
    use iced::{advanced, Rectangle, Task, Vector};

    use crate::widget::Element;
    use crate::widget::{decorate, Renderer};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Key {
        Divider,
        Message(message::Hash),
    }

    impl Key {
        pub fn message(message: &data::Message) -> Self {
            Self::Message(message.hash)
        }
    }

    pub fn keyed<'a, Message: 'a>(
        key: Key,
        inner: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        #[derive(Default)]
        struct State;

        decorate(inner)
            .operate(
                move |_state: &mut State,
                      inner: &Element<'a, Message>,
                      tree: &mut advanced::widget::Tree,
                      layout: advanced::Layout<'_>,
                      renderer: &Renderer,
                      operation: &mut dyn advanced::widget::Operation<()>| {
                    operation.custom(&mut (key, layout.bounds()), None);
                    inner.as_widget().operate(tree, layout, renderer, operation);
                },
            )
            .into()
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Bounds {
        pub scrollable_bounds: ScrollableBounds,
        pub hit_bounds: Rectangle,
        pub prev_bounds: Option<Rectangle>,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct ScrollableBounds {
        pub viewport: Rectangle,
        pub content: Rectangle,
    }

    pub fn find_bounds(scrollable: scrollable::Id, key: Key) -> Task<Bounds> {
        #[derive(Debug, Clone)]
        struct State {
            active: bool,
            key: Key,
            scrollable: scrollable::Id,
            scrollable_bounds: Option<ScrollableBounds>,
            hit_bounds: Option<Rectangle>,
            prev_bounds: Option<Rectangle>,
        }

        impl Operation<State> for State {
            fn scrollable(
                &mut self,
                _state: &mut dyn widget::operation::Scrollable,
                id: Option<&widget::Id>,
                bounds: Rectangle,
                content_bounds: Rectangle,
                _translation: Vector,
            ) {
                if id == Some(&self.scrollable.clone().into()) {
                    self.scrollable_bounds = Some(ScrollableBounds {
                        viewport: bounds,
                        content: content_bounds,
                    });
                    self.active = true;
                } else {
                    self.active = false;
                }
            }

            fn container(
                &mut self,
                _id: Option<&widget::Id>,
                _bounds: Rectangle,
                operate_on_children: &mut dyn FnMut(&mut dyn Operation<State>),
            ) {
                operate_on_children(self)
            }

            fn custom(&mut self, state: &mut dyn std::any::Any, _id: Option<&widget::Id>) {
                if self.active {
                    if let Some((key, bounds)) = state.downcast_ref::<(Key, Rectangle)>() {
                        if self.key == *key {
                            self.hit_bounds = Some(*bounds);
                        } else if self.hit_bounds.is_none() {
                            self.prev_bounds = Some(*bounds);
                        }
                    }
                }
            }

            fn finish(&self) -> widget::operation::Outcome<State> {
                widget::operation::Outcome::Some(self.clone())
            }
        }

        widget::operate(State {
            active: false,
            scrollable,
            key,
            scrollable_bounds: None,
            hit_bounds: None,
            prev_bounds: None,
        })
        .map(|state| {
            state
                .scrollable_bounds
                .zip(state.hit_bounds)
                .map(|(scrollable_bounds, hit_bounds)| Bounds {
                    scrollable_bounds,
                    hit_bounds,
                    prev_bounds: state.prev_bounds,
                })
        })
        .and_then(Task::done)
    }
}
