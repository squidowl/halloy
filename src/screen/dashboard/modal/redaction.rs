use data::{Config, message};
use iced::widget::{button, column, container, operation, text_input};
use iced::{Length, Task, alignment};

use crate::theme;
use crate::widget::{self, Element, text};

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

pub fn view<'a>(state: &'a State, config: &'a Config) -> Element<'a, Message> {
    let content = column![
        text("Reason for redaction"),
        text_input("Enter reason (optional)", &state.reason)
            .id(state.reason_id.clone())
            .on_input(Message::ReasonChanged)
            .padding(8)
            .width(Length::Fill)
            .on_submit(Message::Submit),
        button(
            container(text("Redact"))
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .padding(5)
        .width(widget::modal::button_width(&config.font))
        .style(|theme, status| theme::button::secondary(theme, status, false))
        .on_press(Message::Submit),
    ]
    .spacing(20)
    .align_x(iced::Alignment::Center);

    widget::modal::container(content, &config.font)
        .style(theme::container::tooltip)
        .into()
}
