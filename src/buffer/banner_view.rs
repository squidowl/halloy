use chrono::{DateTime, Utc};
use data::history;
use data::server::Server;
use data::user::Nick;
use data::{Config, User};
use iced::widget::{column, container, row, scrollable, Row};
use iced::{Command, Length};

use super::user_context;
use crate::widget::{selectable_text, Element};
use crate::{theme, Theme};

#[derive(Debug, Clone)]
pub enum Message {
    Scrolled { viewport: scrollable::Viewport },
    UserContext(user_context::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
}

#[derive(Debug, Clone, Copy)]
pub enum Kind<'a> {
    ChannelTopic(&'a Server, &'a str),
}

pub fn view<'a>(
    state: &State,
    kind: Kind,
    history: &'a history::Manager,
    format: impl Fn(&'a data::Message) -> Option<Element<'a, Message>> + 'a,
) -> Element<'a, Message> {
    let Some(history::BannerView { messages }) = (match kind {
        Kind::ChannelTopic(server, channel) => history.get_channel_topic(server, channel),
    }) else {
        return column![].into();
    };

    let messages = messages.into_iter().filter_map(format).collect::<Vec<_>>();

    let padding = if messages.is_empty() { [0, 0] } else { [4, 8] };

    let content = column![column(messages)];

    scrollable(container(content).width(Length::Fill).padding(padding))
        .style(theme::Scrollable::Banner)
        .direction(scrollable::Direction::Vertical(
            scrollable::Properties::default()
                .alignment(scrollable::Alignment::Start)
                .width(5)
                .scroller_width(5),
        ))
        .id(state.scrollable.clone())
        .into()
}

pub fn style_topic_who_time<'a>(
    who: &str,
    time: DateTime<Utc>,
    long_who: Option<&str>,
    users: &[User],
    config: &'a Config,
) -> Row<'a, Message, Theme> {
    if let Some(user) = users.iter().find(|user| user.nickname() == Nick::from(who)) {
        row![]
            .push(selectable_text("set by ").style(theme::Text::Banner))
            .push(
                user_context::view(
                    selectable_text(long_who.unwrap_or(who)).style(theme::Text::Nickname(
                        user.color_seed(&config.buffer.nickname.color),
                        false,
                    )),
                    user.clone(),
                )
                .map(Message::UserContext),
            )
            .push(selectable_text(format!(" at {}", time.to_rfc2822())).style(theme::Text::Banner))
    } else {
        row![]
            .push(selectable_text("set by ").style(theme::Text::Banner))
            .push(selectable_text(long_who.unwrap_or(who)).style(theme::Text::Server))
            .push(selectable_text(format!(" at {}", time.to_rfc2822())).style(theme::Text::Banner))
    }
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
