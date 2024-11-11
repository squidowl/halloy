use chrono::{DateTime, Utc};
use data::user::Nick;
use data::{message, Config, Server, User};
use iced::widget::{column, container, horizontal_rule, row, scrollable, Scrollable};
use iced::Length;

use super::user_context;
use crate::widget::{double_pass, message_content, selectable_text, Element};
use crate::{theme, Theme};

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
    OpenChannel(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    UserContext(user_context::Message),
    Link(message::Link),
}

pub fn update(message: Message) -> Option<Event> {
    match message {
        Message::UserContext(message) => user_context::update(message).map(Event::UserContext),
        Message::Link(message::Link::Channel(channel)) => Some(Event::OpenChannel(channel)),
        Message::Link(message::Link::Url(url)) => {
            let _ = open::that_detached(url);
            None
        }
        Message::Link(message::Link::User(user)) => Some(Event::UserContext(
            user_context::Event::SingleClick(user.nickname().to_owned()),
        )),
        Message::Link(message::Link::GoToMessage(..)) => None,
    }
}

pub fn view<'a>(
    server: &'a Server,
    channel: &'a String,
    content: &'a message::Content,
    who: Option<&'a str>,
    time: Option<&'a DateTime<Utc>>,
    max_lines: u16,
    users: &'a [User],
    our_user: Option<&'a User>,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let set_by = who.and_then(|who| {
        let nick = Nick::from(who.split('!').next()?);

        let user = if let Some(user) = users.iter().find(|user| user.nickname() == nick) {
            user_context::view(
                selectable_text(user.display(config.buffer.channel.nicklist.show_access_levels))
                    .style(|theme| theme::selectable_text::topic_nickname(theme, config, user)),
                server,
                Some(channel),
                user,
                Some(user),
                our_user,
            )
        } else {
            selectable_text(who)
                .style(theme::selectable_text::tertiary)
                .into()
        };

        Some(
            Element::new(row![
                selectable_text("set by ").style(theme::selectable_text::topic),
                user,
                selectable_text(format!(" at {}", time?.to_rfc2822()))
                    .style(theme::selectable_text::topic),
            ])
            .map(Message::UserContext),
        )
    });

    let content = column![message_content(
        content,
        theme,
        Message::Link,
        theme::selectable_text::topic,
        config,
    )]
    .push_maybe(set_by);

    let scrollable = Scrollable::new(container(content).width(Length::Fill).padding(padding()))
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
