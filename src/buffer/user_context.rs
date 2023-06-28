use data::User;
use iced::widget::{button, text};

use crate::theme;
use crate::widget::{context_menu, Element};

#[derive(Debug, Clone, Copy)]
enum Entry {
    Whois,
    Query,
}

impl Entry {
    fn list() -> Vec<Self> {
        vec![Entry::Whois, Entry::Query]
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Whois(User),
    Query(User),
}

#[derive(Debug, Clone)]
pub enum Event {
    SendWhois(User),
    OpenQuery(User),
}

pub fn update(message: Message) -> Event {
    match message {
        Message::Whois(user) => {
            Event::SendWhois(user)
            // let command = data::Command::Whois(None, user.nickname().to_string());

            // let input = data::Input::command(buffer, command);

            // if let Some(encoded) = input.encoded() {
            //     clients.send(input.server(), encoded);
            // }

            // if let Some(message) = clients
            //     .nickname(input.server())
            //     .and_then(|nick| input.message(&nick))
            // {
            //     history.record_message(input.server(), message);
            // }

            // None
        }
        Message::Query(user) => Event::OpenQuery(user),
    }
}

pub fn view<'a>(content: impl Into<Element<'a, Message>>, user: User) -> Element<'a, Message> {
    let entries = Entry::list();

    context_menu(content, entries, move |entry| {
        let (content, message) = match entry {
            Entry::Whois => ("Whois", Message::Whois(user.clone())),
            Entry::Query => ("Message", Message::Query(user.clone())),
        };

        button(text(content).style(theme::Text::Primary))
            // Based off longest entry text
            .width(110)
            .style(theme::Button::Context)
            .on_press(message)
            .into()
    })
}
