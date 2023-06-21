use data::message::Limit;
use data::server::Server;
use data::user::Nick;
use data::{history, time};
use iced::widget::scrollable;
use iced::{Command, Length};

use crate::widget::{Column, Element};

#[derive(Debug, Clone)]
pub enum Message {
    Scrolled {
        count: usize,
        remaining: bool,
        oldest: time::Posix,
        viewport: scrollable::Viewport,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Kind<'a> {
    Server(&'a Server),
    Channel(&'a Server, &'a str),
    Query(&'a Server, &'a Nick),
}

pub fn view<'a>(
    state: &State,
    kind: Kind,
    history: &'a history::Manager,
    format: impl Fn(&'a data::Message) -> Option<Element<'a, Message>> + 'a,
) -> Element<'a, Message> {
    let (total, messages) = match kind {
        Kind::Server(server) => history.get_server_messages(server, Some(state.limit)),
        Kind::Channel(server, channel) => {
            history.get_channel_messages(server, channel, Some(state.limit))
        }
        Kind::Query(server, user) => history.get_query_messages(server, user, Some(state.limit)),
    };

    let count = messages.len();
    let remaining = count < total;
    let oldest = messages
        .first()
        .map(|message| message.datetime.into())
        .unwrap_or_else(time::Posix::now);

    scrollable(
        Column::with_children(messages.into_iter().filter_map(format).collect())
            .width(Length::Fill)
            .padding([0, 8]),
    )
    .vertical_scroll(
        scrollable::Properties::default()
            .alignment(state.status.alignment())
            .width(5)
            .scroller_width(5),
    )
    .on_scroll(move |viewport| Message::Scrolled {
        count,
        remaining,
        oldest,
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
}

impl State {
    pub fn new() -> Self {
        Self {
            scrollable: scrollable::Id::unique(),
            limit: Limit::bottom(),
            status: Status::default(),
        }
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Scrolled {
                count,
                remaining,
                oldest,
                viewport,
            } => {
                let old_status = self.status;
                let relative_offset = viewport.relative_offset().y;

                match old_status {
                    _ if old_status.is_loading_zone(relative_offset) && remaining => {
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
                    _ if old_status.is_bottom_of_scrollable(relative_offset) => {
                        self.status = Status::Idle(Anchor::Bottom);
                        self.limit = Limit::bottom();
                    }
                    _ if old_status.is_top_of_scrollable(relative_offset) => {
                        self.status = Status::Idle(Anchor::Top);
                        self.limit = Limit::top();
                    }
                    Status::Idle(anchor) if !old_status.is_idle_zone(relative_offset) => {
                        self.status = Status::Unlocked(anchor);

                        if matches!(anchor, Anchor::Bottom) {
                            self.limit = Limit::Since(oldest);
                        }
                    }
                    Status::Loading(anchor) => {
                        self.status = Status::Unlocked(anchor);

                        if matches!(anchor, Anchor::Bottom) {
                            self.limit = Limit::Since(oldest);
                        }
                    }
                    Status::Unlocked(_) | Status::Idle(_) => {}
                }

                if let Some(new_offset) = self.status.new_offset(old_status, viewport) {
                    return scrollable::scroll_to(self.scrollable.clone(), new_offset);
                }
            }
        }

        Command::none()
    }

    pub fn scroll_to_start(&mut self) -> Command<Message> {
        self.status = Status::Idle(Anchor::Top);
        self.limit = Limit::top();
        scrollable::scroll_to(
            self.scrollable.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
        )
    }

    pub fn scroll_to_end(&mut self) -> Command<Message> {
        self.status = Status::Idle(Anchor::Bottom);
        self.limit = Limit::bottom();
        scrollable::scroll_to(
            self.scrollable.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
        )
    }
}

#[derive(Debug, Clone, Copy)]
enum Status {
    Idle(Anchor),
    Unlocked(Anchor),
    Loading(Anchor),
}

#[derive(Debug, Clone, Copy)]
enum Anchor {
    Top,
    Bottom,
}

impl Status {
    fn anchor(self) -> Anchor {
        match self {
            Status::Idle(anchor) => anchor,
            Status::Unlocked(anchor) => anchor,
            Status::Loading(anchor) => anchor,
        }
    }

    fn alignment(self) -> scrollable::Alignment {
        match self {
            Status::Idle(anchor) => match anchor {
                Anchor::Top => scrollable::Alignment::Start,
                Anchor::Bottom => scrollable::Alignment::End,
            },
            Status::Unlocked(_) => scrollable::Alignment::Start,
            Status::Loading(anchor) => match anchor {
                Anchor::Top => scrollable::Alignment::Start,
                Anchor::Bottom => scrollable::Alignment::End,
            },
        }
    }

    fn is_loading_zone(self, relative_offset: f32) -> bool {
        match self.anchor() {
            Anchor::Top => self.is_bottom_of_scrollable(relative_offset),
            Anchor::Bottom => self.is_top_of_scrollable(relative_offset),
        }
    }

    fn is_idle_zone(self, relative_offset: f32) -> bool {
        match self.anchor() {
            Anchor::Top => self.is_top_of_scrollable(relative_offset),
            Anchor::Bottom => self.is_bottom_of_scrollable(relative_offset),
        }
    }

    fn is_top_of_scrollable(self, relative_offset: f32) -> bool {
        match self.alignment() {
            scrollable::Alignment::Start => relative_offset == 0.0,
            scrollable::Alignment::End => relative_offset == 1.0,
        }
    }

    fn is_bottom_of_scrollable(self, relative_offset: f32) -> bool {
        match self.alignment() {
            scrollable::Alignment::Start => relative_offset == 1.0,
            scrollable::Alignment::End => relative_offset == 0.0,
        }
    }

    fn new_offset(
        self,
        other: Self,
        viewport: scrollable::Viewport,
    ) -> Option<scrollable::AbsoluteOffset> {
        let old = self.alignment();
        let new = other.alignment();

        let absolute_offset = viewport.absolute_offset();

        if old != new {
            let scrollable::AbsoluteOffset { x, y } = absolute_offset;

            let scroll_height = (viewport.content_bounds.height - viewport.bounds.height).max(0.0);

            Some(scrollable::AbsoluteOffset {
                x,
                y: (scroll_height - y).max(0.0),
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
