use data::{message::Channel, server::Server};
use iced::{
    pure::{container, text, widget::Column, Element},
    Length,
};

use crate::theme::Theme;

pub fn view<'a, Message: 'a>(
    server: &Server,
    channel: &Channel,
    clients: &data::client::Map,
    _theme: &'a Theme,
) -> Element<'a, Message> {
    let messages = clients
        .get_messages(server, channel)
        .into_iter()
        .map(|message| text(format!("{:?}", message)).into())
        .collect();

    let content = Column::with_children(messages);

    // TODO: Scrollable with chat messages.

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
