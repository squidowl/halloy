use chrono::{DateTime, Utc};
use data::{Config, Server, User, isupport, message, target};
use iced::Length;
use iced::widget::{
    Scrollable, column, container, horizontal_rule, row, scrollable,
};

use super::user_context;
use crate::widget::{Element, double_pass, message_content, selectable_text};
use crate::{Theme, theme};

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
    OpenChannel(target::Channel),
    OpenUrl(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    UserContext(user_context::Message),
    Link(message::Link),
}

pub fn update(message: Message) -> Option<Event> {
    match message {
        Message::UserContext(message) => {
            Some(Event::UserContext(user_context::update(message)))
        }
        Message::Link(message::Link::Channel(channel)) => {
            Some(Event::OpenChannel(channel))
        }
        Message::Link(message::Link::Url(url)) => Some(Event::OpenUrl(url)),
        Message::Link(message::Link::User(user)) => Some(Event::UserContext(
            user_context::Event::InsertNickname(user.nickname().to_owned()),
        )),
        Message::Link(message::Link::GoToMessage(..)) => None,
    }
}

pub fn view<'a>(
    server: &'a Server,
    casemapping: isupport::CaseMap,
    channel: &'a target::Channel,
    content: &'a message::Content,
    who: Option<&'a str>,
    time: Option<&'a DateTime<Utc>>,
    max_lines: u16,
    users: &'a [User],
    our_user: Option<&'a User>,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let set_by =
        who.and_then(|who| User::try_from(who).ok())
            .and_then(|user| {
                let channel_user = users.iter().find(|u| **u == user);

                // If user is in channel, we return user_context component.
                // Otherwise selectable_text component.
                let content = if let Some(user) = channel_user {
                    user_context::view(
                        selectable_text(user.nickname().to_string()).style(
                            |theme| {
                                theme::selectable_text::topic_nickname(
                                    theme, config, user,
                                )
                            },
                        ),
                        server,
                        casemapping,
                        Some(channel),
                        user,
                        Some(user),
                        our_user,
                        config,
                        &config.buffer.nickname.click,
                    )
                } else {
                    selectable_text(user.display(false))
                        .style(move |theme| {
                            theme::selectable_text::topic_nickname(
                                theme, config, &user,
                            )
                        })
                        .into()
                };

                Some(
                    Element::new(row![
                        selectable_text("set by ")
                            .style(theme::selectable_text::topic),
                        content,
                        selectable_text(format!(" at {}", time?.to_rfc2822()))
                            .style(theme::selectable_text::topic),
                    ])
                    .map(Message::UserContext),
                )
            });

    let content = column![message_content(
        content,
        casemapping,
        theme,
        Message::Link,
        theme::selectable_text::topic,
        config,
    )]
    .push_maybe(set_by);

    let scrollable = Scrollable::new(
        container(content).width(Length::Fill).padding(padding()),
    )
    .direction(scrollable::Direction::Vertical(
        scrollable::Scrollbar::new().width(1).scroller_width(1),
    ))
    .style(theme::scrollable::hidden);

    // Use double pass to limit layout to `max_lines` of text
    column![
        double_pass(
            container(column((0..max_lines).map(|_| "".into())))
                .width(Length::Fill)
                .padding(padding()),
            column![container(scrollable)].width(Length::Fill),
        ),
        container(horizontal_rule(1))
            .width(Length::Fill)
            .padding([0, 11])
    ]
    .spacing(8)
    .into()
}

fn padding() -> [u16; 2] {
    [0, 8]
}
