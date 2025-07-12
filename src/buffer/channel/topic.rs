use chrono::{DateTime, Local, Utc};
use data::user::{ChannelUsers, Nick};
use data::{Config, Server, User, isupport, message, target};
use iced::Length;
use iced::widget::{
    Scrollable, column, container, horizontal_rule, row, scrollable,
};

use super::user_context;
use crate::widget::{Element, double_pass, message_content, selectable_text};
use crate::{Theme, font, theme};

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
    prefix: &'a [isupport::PrefixMap],
    channel: &'a target::Channel,
    content: &'a message::Content,
    who: Option<Nick>,
    time: Option<&'a DateTime<Utc>>,
    max_lines: u16,
    users: Option<&'a ChannelUsers>,
    our_user: Option<&'a User>,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let set_by = who.map(User::from).and_then(|user| {
        let channel_user = users.and_then(|users| users.resolve(&user));

        // If user is in channel, we return user_context component.
        // Otherwise selectable_text component.
        let content = if let Some(user) = channel_user {
            user_context::view(
                selectable_text(user.nickname().to_string())
                    .font_maybe(
                        theme::font_style::nickname(theme).map(font::get),
                    )
                    .style(|theme| {
                        theme::selectable_text::topic_nickname(
                            theme, config, user,
                        )
                    }),
                server,
                casemapping,
                prefix,
                Some(channel),
                user,
                Some(user),
                our_user,
                config,
                theme,
                &config.buffer.nickname.click,
            )
        } else {
            selectable_text(user.display(false))
                .font_maybe(theme::font_style::nickname(theme).map(font::get))
                .style(move |theme| {
                    theme::selectable_text::topic_nickname(theme, config, &user)
                })
                .into()
        };

        Some(
            Element::new(row![
                selectable_text("set by ")
                    .font_maybe(theme::font_style::topic(theme).map(font::get))
                    .style(theme::selectable_text::topic),
                content,
                selectable_text(format!(
                    " at {}",
                    time?.with_timezone(&Local).to_rfc2822()
                ))
                .font_maybe(theme::font_style::topic(theme).map(font::get))
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
        theme::font_style::topic,
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
