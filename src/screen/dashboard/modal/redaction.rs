use data::{Config, message};
use iced::widget::{button, column, operation, text_input};
use iced::{Length, Task};

use crate::widget::{Element, text};

const MODAL_WIDTH: f32 = 380.0;
const MODAL_HEIGHT: f32 = 130.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    msgid: message::Id,
    reason_id: iced::widget::Id,
    reason: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    ReasonChanged(String),
    Submit,
}

#[derive(Debug, Clone)]
pub enum Event {
    RedactReason { msgid: message::Id, reason: String },
}

impl State {
    pub fn new(msgid: message::Id) -> Self {
        Self {
            msgid,
            reason_id: iced::widget::Id::unique(),
            reason: String::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::ReasonChanged(reason) => {
                self.reason = reason;
                None
            }
            Message::Submit => Some(Event::RedactReason {
                msgid: self.msgid.clone(),
                reason: self.reason.clone(),
            }),
        }
    }

    pub fn focus(&self) -> Task<Message> {
        let reason_id = self.reason_id.clone();

        operation::is_focused(reason_id.clone()).then(move |is_focused| {
            if is_focused {
                Task::none()
            } else {
                operation::focus(reason_id.clone())
            }
        })
    }
}

pub fn view<'a>(state: &'a State, _config: &'a Config) -> Element<'a, Message> {
    let content = column![
        text("Reason for redaction:").size(16),
        text_input("Enter reason", &state.reason)
            .id(state.reason_id.clone())
            .on_input(Message::ReasonChanged)
            .padding(8)
            .width(Length::Fill)
            .on_submit(Message::Submit),
        button("Redact")
            .padding(8)
            .width(Length::Fill)
            .on_press(Message::Submit),
    ]
    .spacing(12)
    .height(Length::Fill);

    iced::widget::container(content)
        .width(Length::Fixed(MODAL_WIDTH))
        .height(Length::Fixed(MODAL_HEIGHT))
        .padding(8)
        .style(crate::theme::container::tooltip)
        .into()
}
