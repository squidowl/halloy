use chrono::{DateTime, Utc};
use data::user::Nick;
use data::{Buffer, Config, User};
use iced::widget::{column, container, horizontal_rule, row, scrollable, Scrollable};
use iced::Length;

use super::user_context;
use crate::theme;
use crate::widget::{double_pass, selectable_text, Element};

pub fn view<'a>(
    text: &'a str,
    who: Option<&'a str>,
    time: Option<&'a DateTime<Utc>>,
    max_lines: u16,
    users: &'a [User],
    buffer: &Buffer,
    our_user: Option<&'a User>,
    config: &'a Config,
) -> Element<'a, user_context::Message> {
    let set_by = who.and_then(|who| {
        let nick = Nick::from(who.split('!').next()?);

        let user = if let Some(user) = users.iter().find(|user| user.nickname() == nick) {
            user_context::view(
                selectable_text(who).style(|theme| {
                    theme::selectable_text::nickname(
                        theme,
                        user.color_seed(&config.buffer.nickname.color),
                        false,
                    )
                }),
                user,
                Some(Some(user)),
                buffer.clone(),
                our_user,
            )
        } else {
            selectable_text(who)
                .style(theme::selectable_text::info)
                .into()
        };

        Some(row![
            selectable_text("set by ").style(theme::selectable_text::transparent),
            user,
            selectable_text(format!(" at {}", time?.to_rfc2822()))
                .style(theme::selectable_text::transparent),
        ])
    });

    let content = column![selectable_text(text).style(theme::selectable_text::transparent)]
        .push_maybe(set_by);

    let scrollable = Scrollable::with_direction(
        container(content).width(Length::Fill).padding(padding()),
        scrollable::Direction::Vertical(scrollable::Properties::new().width(1).scroller_width(1)),
    )
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
