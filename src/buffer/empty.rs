use core::fmt;

use data::server::Server;
use iced::widget::{button, column, container, horizontal_space, row, text, vertical_space};
use iced::{alignment, Length};

use crate::{icon, theme, widget::Element};

#[derive(Debug, Clone)]
pub enum Message {
    SelectChannel((Server, String)),
    SelectServer(Server),
}

#[derive(Debug, Clone)]
pub enum Event {
    SelectChannel((Server, String)),
    SelectServer(Server),
}

pub fn view<'a>(_state: &State, _clients: &data::client::Map) -> Element<'a, Message> {
    // let mut column = column![].spacing(1);

    // for (server, channels) in clients.get_channels().iter() {
    //     column = column.push(
    //         button(
    //             row![]
    //                 .push(icon::house())
    //                 .push(horizontal_space(5))
    //                 .push(text(server.to_string())),
    //         )
    //         .style(theme::Button::Primary)
    //         .on_press(Message::SelectServer(server.clone())),
    //     );

    //     for channel in channels {
    //         column = column.push(
    //             button(
    //                 row![]
    //                     .push(icon::chat())
    //                     .push(horizontal_space(5))
    //                     .push(text(channel)),
    //             )
    //             .style(theme::Button::Primary)
    //             .on_press(Message::SelectChannel((server.clone(), channel.clone()))),
    //         );
    //     }

    //     column = column.push(vertical_space(Length::Units(10)));
    // }

    // container(column)
    //     .align_x(alignment::Horizontal::Center)
    //     .align_y(alignment::Vertical::Center)
    //     .width(Length::Fill)
    //     .height(Length::Fill)
    //     .into()

    container(text("Welcome to Halloy"))
        .style(theme::Container::Pane { selected: false })
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[derive(Debug, Default, Clone)]
pub struct State {}

impl State {
    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::SelectChannel((server, channel)) => {
                Some(Event::SelectChannel((server, channel)))
            }
            Message::SelectServer(server) => Some(Event::SelectServer(server)),
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Not sure if this should be empty.
        write!(f, "Hello world")
    }
}
