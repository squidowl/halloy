use data::User;
use iced::widget::text::LineHeight;
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
    SingleClick(User),
}

#[derive(Debug, Clone)]
pub enum Event {
    SendWhois(User),
    OpenQuery(User),
    SingleClick(User),
}

pub fn update(message: Message) -> Event {
    match message {
        Message::Whois(user) => Event::SendWhois(user),
        Message::Query(user) => Event::OpenQuery(user),
        Message::SingleClick(user) => Event::SingleClick(user),
    }
}

pub fn view<'a>(content: impl Into<Element<'a, Message>>, user: User, font: &data::config::Font) -> Element<'a, Message> {
    let font = font.clone();
    let entries = Entry::list();

    let content = button(content)
        .padding(0)
        .style(theme::Button::Bare)
        .on_press(Message::SingleClick(user.clone()));

    context_menu(content, entries, move |entry, length| {
        let (content, message) = match entry {
            Entry::Whois => ("Whois", Message::Whois(user.clone())),
            Entry::Query => ("Message", Message::Query(user.clone())),
        };

        // Height based on font size, with some margin added.
        let height = LineHeight::default().to_absolute((font.size + 8.0).into());

        button(text(content).style(theme::Text::Primary))
            .width(length)
            .height(height)
            .style(theme::Button::Context)
            .on_press(message)
            .into()
    })
}
