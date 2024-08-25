use chrono::{DateTime, Utc};
use data::user::Nick;
use data::{message, Buffer, Config, User};
use iced::widget::{column, container, horizontal_rule, row, scrollable, Scrollable};
use iced::Length;

use super::user_context;
use crate::widget::{double_pass, message_content, selectable_text, Element};
use crate::{theme, Theme};

pub fn view<'a>(
    content: &'a message::Content,
    who: Option<&'a str>,
    time: Option<&'a DateTime<Utc>>,
    max_lines: u16,
    users: &'a [User],
    buffer: &Buffer,
    our_user: Option<&'a User>,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, user_context::Message> {
    let set_by = who.and_then(|who| {
        let nick = Nick::from(who.split('!').next()?);

        let user = if let Some(user) = users.iter().find(|user| user.nickname() == nick) {
            user_context::view(
                selectable_text(who).style(|theme| {
                    theme::selectable_text::nickname(
                        theme,
                        user.nick_color(theme.colors(), &config.buffer.nickname.color),
                        false,
                        config.buffer.nickname.away_transparency,
                    )
                }),
                user,
                Some(user),
                buffer.clone(),
                our_user,
            )
        } else {
            selectable_text(who)
                .style(theme::selectable_text::tertiary)
                .into()
        };

        Some(row![
            selectable_text("set by ").style(theme::selectable_text::topic),
            user,
            selectable_text(format!(" at {}", time?.to_rfc2822()))
                .style(theme::selectable_text::topic),
        ])
    });

    let content = column![message_content(
        content,
        theme,
        user_context::Message::Link,
        theme::selectable_text::topic
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
