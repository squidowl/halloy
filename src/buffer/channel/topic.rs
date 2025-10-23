use chrono::{DateTime, Local, Utc};
use data::user::{ChannelUsers, NickRef};
use data::{Config, Server, User, isupport, message, target};
use iced::widget::{Scrollable, column, container, row, rule, scrollable};
use iced::{Color, Length, padding};

use super::context_menu;
use crate::widget::{Element, double_pass, message_content, selectable_text};
use crate::{Theme, font, theme};

#[derive(Debug, Clone)]
pub enum Event {
    ContextMenu(context_menu::Event),
    OpenChannel(target::Channel),
    OpenUrl(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    ContextMenu(context_menu::Message),
    Link(message::Link),
}

pub fn update(message: Message) -> Option<Event> {
    match message {
        Message::ContextMenu(message) => {
            Some(Event::ContextMenu(context_menu::update(message)))
        }
        Message::Link(message::Link::Channel(channel)) => {
            Some(Event::OpenChannel(channel))
        }
        Message::Link(message::Link::Url(url)) => Some(Event::OpenUrl(url)),
        Message::Link(message::Link::User(user)) => Some(Event::ContextMenu(
            context_menu::Event::InsertNickname(user.nickname().to_owned()),
        )),
        Message::Link(message::Link::GoToMessage(..)) => None,
    }
}

pub fn view<'a>(
    server: &'a Server,
    chantypes: &'a [char],
    casemapping: isupport::CaseMap,
    prefix: &'a [isupport::PrefixMap],
    channel: &'a target::Channel,
    content: &'a message::Content,
    who: Option<NickRef<'a>>,
    time: Option<&'a DateTime<Utc>>,
    max_lines: u16,
    users: Option<&'a ChannelUsers>,
    our_user: Option<&'a User>,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let set_by = who.map(NickRef::to_owned).map(User::from).and_then(|user| {
        let channel_user = users.and_then(|users| users.resolve(&user));

        // If user is in channel, we return user_context component.
        // Otherwise selectable_text component.
        let content = if let Some(user) = channel_user {
            context_menu::user(
                selectable_text(user.nickname().to_string())
                    .font_maybe(
                        theme::font_style::nickname(theme, false)
                            .map(font::get),
                    )
                    .style(|theme| {
                        theme::selectable_text::topic_nickname(
                            theme, config, user,
                        )
                    }),
                server,
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
            selectable_text(user.display(false, None))
                .font_maybe(
                    theme::font_style::nickname(theme, false).map(font::get),
                )
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
            .map(Message::ContextMenu),
        )
    });

    let content = column![
        message_content(
            content,
            chantypes,
            casemapping,
            theme,
            Message::Link,
            theme::selectable_text::topic,
            theme::font_style::topic,
            Option::<fn(Color) -> Color>::None,
            config,
        ),
        set_by
    ];

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
        container(rule::horizontal(1))
            .width(Length::Fill)
            .padding([0, 11])
    ]
    .padding(padding::top(4))
    .spacing(8)
    .into()
}

fn padding() -> [u16; 2] {
    [0, 8]
}
