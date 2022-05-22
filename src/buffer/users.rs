use core::fmt;

use iced::{alignment, pure::Element, Length};
use iced_pure::{
    button, column, container, horizontal_space, row, scrollable, text, vertical_space,
};

use crate::{icon, style, theme::Theme};

#[derive(Debug, Clone)]
pub enum Message {
    Noop,
}

pub fn view<'a>(_state: &State, theme: &'a Theme) -> Element<'a, Message> {
    // TODO: Get open channels. For now just dummy hardcoded to work out UI.
    // TODO: Rewrite to function to reduce repetivness.

    let mut column = column().width(Length::Fill).spacing(1);

    for _ in 0..15 {
        column = column.push(
            button(text("Foobar"))
                .width(Length::Fill)
                .style(style::button::secondary(theme))
                .on_press(Message::Noop),
        );
    }

    let content = scrollable(column).height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Default, Clone)]
pub struct State {}

impl State {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Noop => todo!(),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Users in |TODO|")
    }
}
