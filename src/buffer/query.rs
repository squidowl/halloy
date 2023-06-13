use core::fmt;

use data::{Server, User};
use iced::widget::{container, text};
use iced::{alignment, Command, Length};

use crate::theme;
use crate::widget::{input, Element};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(_state: &Query, _clients: &data::client::Map) -> Element<'a, Message> {
    container(text("Welcome to Halloy"))
        .style(theme::Container::PaneBody { selected: false })
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[derive(Debug, Clone)]
pub struct Query {
    pub server: Server,
    pub user: User,
    input_id: input::Id,
}

impl Query {
    pub fn new(server: Server, user: User) -> Self {
        Self {
            server,
            user,
            input_id: input::Id::unique(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
    ) -> (Command<Message>, Option<Event>) {
        // match message {}

        (Command::none(), None)
    }

    pub fn focus(&self) -> Command<Message> {
        input::focus(self.input_id.clone())
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user.formatted())
    }
}
