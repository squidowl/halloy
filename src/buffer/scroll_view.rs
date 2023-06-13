use data::client;
use data::message::Limit;
use data::server::Server;
use iced::widget::scrollable;
use iced::{Command, Length};

use crate::widget::{Column, Element};

#[derive(Debug, Clone)]
pub enum Message {
    Scrolled(scrollable::Viewport),
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
    let messages = match kind {
        Kind::Server(server) => clients.get_server_messages(server, Some(state.limit)),
        Kind::Channel(server, channel) => {
            clients.get_channel_messages(server, channel, Some(state.limit))
        }
    };
    let at_limit = messages.len() == state.limit.value();

    let messages: Vec<_> = messages.into_iter().filter_map(format).collect();

    let mut scrollable = scrollable(
        Column::with_children(messages)
            .width(Length::Fill)
            .padding([0, 8]),
    )
    .vertical_scroll(scrollable::Properties::default().alignment(scrollable::Alignment::End))
    .id(state.scrollable.clone());

    if at_limit {
        scrollable = scrollable.on_scroll(Message::Scrolled)
    }

    scrollable.into()
}

#[derive(Debug, Clone)]
pub struct State {
    pub scrollable: scrollable::Id,
    limit: Limit,
}

impl State {
    pub fn new() -> Self {
        Self {
            scrollable: scrollable::Id::unique(),
            limit: Limit::default(),
        }
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Scrolled(_viewport) => {
                // TODO: Need a proper reversed scrollable to get this
                // behavior working
                // if viewport.absolute_offset().y == 0.0 {
                //     self.limit.increase(Limit::DEFAULT_STEP)
                // }

                Command::none()
            }
        }
    }
}
