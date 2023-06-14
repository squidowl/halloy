use data::message::Limit;
use data::server::Server;
use data::{client, time};
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
}

pub fn view<'a>(
    state: &State,
    kind: Kind,
    clients: &'a client::Map,
    format: impl Fn(&'a data::Message) -> Option<Element<'a, Message>> + 'a,
) -> Element<'a, Message> {
    let (total, messages) = match kind {
        Kind::Server(server) => clients.get_server_messages(server, Some(state.limit)),
        Kind::Channel(server, channel) => {
            clients.get_channel_messages(server, channel, Some(state.limit))
        }
    };

    let count = messages.len();
    let remaining = count < total;
    let oldest = messages
        .first()
        .map(|message| message.timestamp)
        .unwrap_or_else(|| time::Posix::now());

    scrollable(
        Column::with_children(messages.into_iter().filter_map(format).collect())
            .width(Length::Fill)
            .padding([0, 8]),
    )
    .vertical_scroll(scrollable::Properties::default().alignment(state.anchor.alignment()))
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
    anchor: Anchor,
}

impl State {
    pub fn new() -> Self {
        Self {
            scrollable: scrollable::Id::unique(),
            limit: Limit::default(),
            anchor: Anchor::default(),
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
                let old_anchor = self.anchor;
                let relative_offset = viewport.relative_offset().y;

                match old_anchor {
                    _ if old_anchor.is_top(relative_offset) && remaining => {
                        self.anchor = Anchor::Loading;
                        self.limit = Limit::Bottom(count + Limit::DEFAULT_STEP);
                    }
                    _ if old_anchor.is_bottom(relative_offset) => {
                        self.anchor = Anchor::Bottom;
                        self.limit = Limit::default();
                    }
                    Anchor::Bottom if !old_anchor.is_bottom(relative_offset) => {
                        self.anchor = Anchor::Unlocked;
                        self.limit = Limit::Since(oldest);
                    }
                    Anchor::Loading => {
                        self.anchor = Anchor::Unlocked;
                        self.limit = Limit::Since(oldest);
                    }
                    Anchor::Unlocked | Anchor::Bottom => {}
                }

                if let Some(new_offset) = self.anchor.new_offset(old_anchor, viewport) {
                    return scrollable::scroll_to(self.scrollable.clone(), new_offset);
                }
            }
        }

        Command::none()
    }

    pub fn scroll_to_end(&mut self) -> Command<Message> {
        self.anchor = Anchor::Bottom;
        self.limit = Limit::default();
        scrollable::scroll_to(
            self.scrollable.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
        )
    }
}

#[derive(Debug, Clone, Copy)]
enum Anchor {
    // TODO: Add top anchor (cmd + home behavior)
    Bottom,
    Unlocked,
    Loading,
}

impl Anchor {
    fn alignment(self) -> scrollable::Alignment {
        match self {
            Anchor::Bottom => scrollable::Alignment::End,
            Anchor::Unlocked => scrollable::Alignment::Start,
            Anchor::Loading => scrollable::Alignment::End,
        }
    }

    fn is_top(self, relative_offset: f32) -> bool {
        match self.alignment() {
            scrollable::Alignment::Start => relative_offset == 0.0,
            scrollable::Alignment::End => relative_offset == 1.0,
        }
    }

    fn is_bottom(self, relative_offset: f32) -> bool {
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

impl Default for Anchor {
    fn default() -> Self {
        Self::Bottom
    }
}
