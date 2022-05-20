use std::fmt;

use iced::{
    pure::{self, column, container, widget::Column, Element},
    Length,
};

use crate::{style, theme::Theme, widget::sticky_scrollable::scrollable};

#[derive(Debug, Clone)]
pub enum Message {}

pub fn view<'a>(
    clients: &data::client::Map,
    _is_focused: bool,
    _theme: &'a Theme,
) -> Element<'a, Message> {
    let messages: Vec<Element<'a, Message>> = clients
        .get_messages_for_server()
        .into_iter()
        .filter_map(|message| match message.command() {
            data::message::Command::Response { response, text } => response
                .parse(text)
                .map(|value| container(pure::text(value).size(style::TEXT_SIZE)).into()),
            data::message::Command::Notice { text, .. } => {
                Some(container(pure::text(text).size(style::TEXT_SIZE)).into())
            }
            _ => None,
        })
        .collect();

    container(scrollable(
        Column::with_children(messages).width(Length::Fill),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .padding([0, 8])
    .into()
}

#[derive(Debug, Clone)]
pub struct State;

impl State {
    pub fn _update(&mut self, _message: Message, _clients: &data::client::Map) {}
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Server")
    }
}
