use chrono::{DateTime, Local, Utc};
use data::user::ChannelUsers;
use data::{Config, Server, User, isupport, message, target};
use iced::widget::{Scrollable, column, container, row, rule, scrollable};
use iced::{Color, Length, padding};

use super::context_menu::{self, Context};
use crate::widget::{Element, double_pass, message_content, selectable_text};
use crate::{Theme, font, theme};

#[derive(Debug, Clone)]
pub enum Event {
    ContextMenu(context_menu::Event),
    OpenChannel(Server, target::Channel),
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
        Message::Link(message::Link::Channel(server, channel)) => {
            Some(Event::OpenChannel(server, channel))
        }
        Message::Link(message::Link::Url(url)) => Some(Event::OpenUrl(url)),
        Message::Link(message::Link::User(_, user)) => {
            Some(Event::ContextMenu(context_menu::Event::InsertNickname(
                user.nickname().to_owned(),
            )))
        }
        Message::Link(message::Link::GoToMessage(..))
        | Message::Link(message::Link::ExpandCondensedMessage(..))
        | Message::Link(message::Link::ContractCondensedMessage(..)) => None,
    }
}

pub fn view<'a>(
    server: &'a Server,
    chantypes: &'a [char],
    casemapping: isupport::CaseMap,
    prefix: &'a [isupport::PrefixMap],
    channel: &'a target::Channel,
    content: &'a message::Content,
    who: Option<&'a User>,
    time: Option<&'a DateTime<Utc>>,
    max_lines: u16,
    users: Option<&'a ChannelUsers>,
    our_user: Option<&'a User>,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let set_by = who.and_then(|user| {
        let user_in_channel = users.and_then(|users| users.resolve(user));

        // If user is in channel, we return user_context component.
        // Otherwise selectable_text component.
        let content = context_menu::user(
            selectable_text(user.nickname().to_string())
                .font_maybe(
                    theme::font_style::nickname(theme, false).map(font::get),
                )
                .style(move |theme| {
                    theme::selectable_text::topic_nickname(
                        theme,
                        config,
                        user,
                        user_in_channel.is_none(),
                    )
                }),
            server,
            prefix,
            Some(channel),
            user,
            user_in_channel,
            our_user,
            config,
            theme,
            &config.buffer.nickname.click,
        );

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
        message_content::with_context(
            content,
            server,
            chantypes,
            casemapping,
            theme,
            Message::Link,
            None,
            theme::selectable_text::topic,
            theme::font_style::topic,
            Option::<fn(Color) -> Color>::None,
            move |link| match link {
                message::Link::User(_, user) => {
                    let user_in_channel =
                        users.and_then(|users| users.resolve(user));

                    context_menu::Entry::user_list(
                        true,
                        user_in_channel,
                        our_user,
                        config.file_transfer.enabled,
                    )
                }
                message::Link::Url(_) => context_menu::Entry::url_list(),
                _ => vec![],
            },
            move |link, entry, length| {
                let link_context = if let Some(user) = link.user() {
                    let current_user =
                        users.and_then(|users| users.resolve(user));

                    Some(Context::User {
                        server,
                        prefix,
                        channel: Some(channel),
                        user,
                        current_user,
                    })
                } else {
                    link.url().map(Context::Url)
                };

                entry
                    .view(link_context, length, config, theme)
                    .map(Message::ContextMenu)
            },
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
