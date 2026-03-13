pub mod reaction;

use data::{Config, message};

use crate::widget::Element;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    AddReaction(reaction::State),
}

#[derive(Debug, Clone)]
pub enum Message {
    Reaction(reaction::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    React { msgid: message::Id, text: String },
}

impl Modal {
    pub fn update(&mut self, message: Message) -> Option<Event> {
        match (self, message) {
            (Modal::AddReaction(state), Message::Reaction(message)) => state
                .update(message)
                .map(|reaction::Event::React { msgid, text }| Event::React {
                    msgid,
                    text,
                }),
        }
    }

    pub fn view<'a>(&'a self, config: &'a Config) -> Element<'a, Message> {
        match self {
            Modal::AddReaction(state) => {
                reaction::view(state, config).map(Message::Reaction)
            }
        }
    }
}
