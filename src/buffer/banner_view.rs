use data::client::Topic;
use data::user::Nick;
use data::{Config, User};
use iced::widget::{column, container, row, scrollable};
use iced::{Command, Length};

use super::user_context;
use crate::theme;
use crate::widget::{selectable_text, Collection, Element};

#[derive(Debug, Clone)]
pub enum Message {
    Scrolled { viewport: scrollable::Viewport },
    UserContext(user_context::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
}

pub fn view<'a>(
    state: &State,
    topic: &Topic,
    users: &[User],
    config: &'a Config,
) -> Option<Element<'a, Message>> {
    let set_by = if let Some(who) = topic.who.clone() {
        let nick = Nick::from(who.split('!').next()?);

        if let Some(user) = users.iter().find(|user| user.nickname() == nick) {
            Some(
                row![]
                    .push(selectable_text("set by ").style(theme::Text::Banner))
                    .push(
                        user_context::view(
                            selectable_text(who).style(theme::Text::Nickname(
                                user.color_seed(&config.buffer.nickname.color),
                                false,
                            )),
                            user.clone(),
                        )
                        .map(Message::UserContext),
                    )
                    .push(
                        selectable_text(format!(" at {}", topic.time?.to_rfc2822()))
                            .style(theme::Text::Banner),
                    ),
            )
        } else {
            Some(
                row![]
                    .push(selectable_text("set by ").style(theme::Text::Banner))
                    .push(selectable_text(who).style(theme::Text::Server))
                    .push(
                        selectable_text(format!(" at {}", topic.time?.to_rfc2822()))
                            .style(theme::Text::Banner),
                    ),
            )
        }
    } else {
        None
    };

    let content = column![]
        .push(row![].push(selectable_text(topic.text.clone()?).style(theme::Text::Banner)))
        .push_maybe(set_by);

    Some(
        scrollable(container(content).width(Length::Fill).padding(padding()))
            .style(theme::Scrollable::Banner)
            .direction(scrollable::Direction::Vertical(
                scrollable::Properties::default()
                    .alignment(scrollable::Alignment::Start)
                    .width(5)
                    .scroller_width(5),
            ))
            .id(state.scrollable.clone())
            .into(),
    )
}

pub fn padding() -> [u16; 2] {
    [4, 8]
}

#[derive(Debug, Clone)]
pub struct State {
    pub scrollable: scrollable::Id,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scrollable: scrollable::Id::unique(),
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, message: Message) -> (Command<Message>, Option<Event>) {
        if let Message::UserContext(message) = message {
            return (
                Command::none(),
                Some(Event::UserContext(user_context::update(message))),
            );
        }

        (Command::none(), None)
    }
}
