use std::fmt;

use iced::widget::{column, container, scrollable, text, vertical_space};
use iced::{Command, Length};

use crate::widget::{input, Collection, Column, Element};

#[derive(Debug, Clone)]
pub enum Message {
    Send(input::Content),
    CompletionSelected,
}

#[derive(Debug, Clone)]
pub enum Event {}

pub fn view<'a>(
    state: &'a Server,
    clients: &data::client::Map,
    is_focused: bool,
) -> Element<'a, Message> {
    let messages: Vec<Element<'a, Message>> = clients
        .get_server_messages(&state.server)
        .into_iter()
        .filter_map(|message| Some(container(text(message.text()?)).into()))
        .collect();

    let messages = container(
        scrollable(Column::with_children(messages).width(Length::Fill))
            .id(state.scrollable.clone()),
    )
    .height(Length::Fill);
    let spacing = is_focused.then_some(vertical_space(4));
    let text_input = is_focused.then(|| {
        input(
            state.input_id.clone(),
            Message::Send,
            Message::CompletionSelected,
        )
    });

    let scrollable = column![messages]
        .push_maybe(spacing)
        .push_maybe(text_input)
        .height(Length::Fill);

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone)]
pub struct Server {
    pub server: data::server::Server,
    pub scrollable: scrollable::Id,
    input_id: input::Id,
}

impl Server {
    pub fn new(server: data::server::Server) -> Self {
        Self {
            server,
            input_id: input::Id::unique(),
            scrollable: scrollable::Id::unique(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::Send(content) => {
                if let input::Content::Command(command) = content {
                    clients.send_command(&self.server, command);
                    (
                        scrollable::snap_to(
                            self.scrollable.clone(),
                            scrollable::RelativeOffset::END,
                        ),
                        None,
                    )
                } else {
                    (Command::none(), None)
                }
            }
            Message::CompletionSelected => {
                return (input::move_cursor_to_end(self.input_id.clone()), None);
            }
        }
    }

    pub fn focus(&self) -> Command<Message> {
        input::focus(self.input_id.clone())
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.server)
    }
}
