pub mod reaction;
pub mod redaction;

use std::borrow::Cow;

use data::{Config, message};
use iced::Task;

use crate::widget::Element;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    AddReaction(reaction::State),
    RedactReason(redaction::State),
}

#[derive(Debug, Clone)]
pub enum Message {
    Reaction(reaction::Message),
    Redaction(redaction::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    ToggleReaction {
        msgid: message::Id,
        text: Cow<'static, str>,
        unreact: bool,
    },
    RedactReason {
        msgid: message::Id,
        reason: String,
    },
}

impl Modal {
    pub fn update(&mut self, message: Message) -> Option<Event> {
        match (self, message) {
            (Modal::AddReaction(state), Message::Reaction(message)) => {
                state.update(message).map(
                    |reaction::Event::Toggle {
                         msgid,
                         text,
                         unreact,
                     }| Event::ToggleReaction {
                        msgid,
                        text,
                        unreact,
                    },
                )
            }
            (Modal::RedactReason(state), Message::Redaction(reason)) => state
                .update(reason)
                .map(|redaction::Event::RedactReason { msgid, reason }| {
                    Event::RedactReason { msgid, reason }
                }),
            _ => None,
        }
    }

    pub fn view<'a>(&'a self, config: &'a Config) -> Element<'a, Message> {
        match self {
            Modal::AddReaction(state) => {
                reaction::view(state, config).map(Message::Reaction)
            }
            Modal::RedactReason(state) => {
                redaction::view(state, config).map(Message::Redaction)
            }
        }
    }

    pub fn focus(&self) -> Task<Message> {
        match self {
            Modal::AddReaction(state) => state.focus().map(Message::Reaction),
            Modal::RedactReason(state) => state.focus().map(Message::Redaction),
        }
    }
}
