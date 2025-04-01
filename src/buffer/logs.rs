use data::dashboard::BufferAction;
use data::target::Target;
use data::{client, history, isupport, message, Config};
use iced::widget::container;
use iced::{Length, Task};

use super::{scroll_view, user_context};
use crate::widget::{message_content, Element};
use crate::{theme, Theme};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
}

pub enum Event {
    UserContext(user_context::Event),
    OpenBuffer(Target, BufferAction),
    History(Task<history::manager::Message>),
}

pub fn view<'a>(
    state: &'a Logs,
    history: &'a history::Manager,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Logs,
            history,
            None,
            None,
            config,
            move |message, _, _| match message.target.source() {
                message::Source::Internal(message::source::Internal::Logs) => Some(
                    container(message_content(
                        &message.content,
                        isupport::CaseMap::default(),
                        theme,
                        scroll_view::Message::Link,
                        theme::selectable_text::default,
                        config,
                    ))
                    .into(),
                ),
                _ => None,
            },
        )
        .map(Message::ScrollView),
    )
    .height(Length::Fill);

    container(messages)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone, Default)]
pub struct Logs {
    pub scroll_view: scroll_view::State,
}

impl Logs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(
        &mut self,
        message: Message,
        history: &history::Manager,
        clients: &client::Map,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(
                    message,
                    false,
                    scroll_view::Kind::Logs,
                    history,
                    clients,
                    config,
                );

                let event = event.and_then(|event| match event {
                    scroll_view::Event::UserContext(event) => Some(Event::UserContext(event)),
                    scroll_view::Event::OpenBuffer(target, buffer_action) => {
                        Some(Event::OpenBuffer(target, buffer_action))
                    }
                    scroll_view::Event::GoToMessage(_, _, _) => None,
                    scroll_view::Event::RequestOlderChatHistory => None,
                    scroll_view::Event::PreviewChanged => None,
                    scroll_view::Event::HidePreview(..) => None,
                });

                (command.map(Message::ScrollView), event)
            }
        }
    }
}
