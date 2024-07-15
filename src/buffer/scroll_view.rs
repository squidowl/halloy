use data::message::Limit;
use data::server::Server;
use data::user::Nick;
use data::{history, time, Config};
use iced::widget::{column, container, horizontal_rule, row, scrollable, text, Scrollable};
use iced::{padding, Length, Task};

use super::user_context;
use crate::theme;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub enum Message {
    Scrolled {
        count: usize,
        remaining: bool,
        oldest: time::Posix,
        status: Status,
        viewport: scrollable::Viewport,
    },
    UserContext(user_context::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
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
    config: &'a Config,
    format: impl Fn(&'a data::Message) -> Option<Element<'a, Message>> + 'a,
) -> Element<'a, Message> {
    let Some(history::View {
        total,
        old_messages,
        new_messages,
    }) = (match kind {
        Kind::Server(server) => {
            history.get_server_messages(server, Some(state.limit), &config.buffer)
        }
        Kind::Channel(server, channel) => {
            history.get_channel_messages(server, channel, Some(state.limit), &config.buffer)
        }
        Kind::Query(server, user) => {
            history.get_query_messages(server, user, Some(state.limit), &config.buffer)
        }
    })
    else {
        return column![].into();
    };

    let count = old_messages.len() + new_messages.len();
    let remaining = count < total;
    let oldest = old_messages
        .iter()
        .chain(&new_messages)
        .next()
        .map(|message| message.received_at)
        .unwrap_or_else(time::Posix::now);
    let status = state.status;

    let old = old_messages
        .into_iter()
        .filter_map(&format)
        .collect::<Vec<_>>();
    let new = new_messages
        .into_iter()
        .filter_map(format)
        .collect::<Vec<_>>();

    let show_divider = !new.is_empty() || matches!(status, Status::Idle(Anchor::Bottom));

    let content = if show_divider {
        let font_size = config.font.size.map(f32::from).unwrap_or(theme::TEXT_SIZE) - 1.0;

        let divider = row![
            container(horizontal_rule(1))
                .width(Length::Fill)
                .padding(padding::right(6)),
            text("backlog")
                .size(font_size)
                .style(theme::text::transparent),
            container(horizontal_rule(1))
                .width(Length::Fill)
                .padding(padding::left(6))
        ]
        .padding(2)
        .align_y(iced::Alignment::Center);

        column![column(old), divider, column(new)]
    } else {
        column![column(old), column(new)]
    };

    Scrollable::new(container(content).width(Length::Fill).padding([0, 8]))
        .direction(scrollable::Direction::Vertical {
            scrollbar: scrollable::Scrollbar::default()
                .anchor(status.alignment())
                .width(5)
                .scroller_width(5),
            spacing: None,
        })
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
}

impl Default for State {
    fn default() -> Self {
        Self {
            scrollable: scrollable::Id::unique(),
            limit: Limit::bottom(),
            status: Status::default(),
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Event>) {
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
                }
            }
            Message::UserContext(message) => {
                return (
                    Task::none(),
                    Some(Event::UserContext(user_context::update(message))),
                );
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
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Idle(Anchor),
    Unlocked(Anchor),
    Loading(Anchor),
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
