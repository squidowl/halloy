use chrono::{DateTime, Utc};
use data::user::Nick;
use data::{Config, User};
use iced::widget::{column, container, horizontal_rule, row, scrollable, vertical_space};
use iced::Length;

use super::user_context;
use crate::theme;
use crate::widget::{double_pass, selectable_text, Collection, Element};

pub fn view<'a>(
    text: &'a str,
    who: Option<&'a str>,
    time: Option<&'a DateTime<Utc>>,
    max_lines: u16,
    users: &[User],
    config: &'a Config,
) -> Element<'a, user_context::Message> {
    let set_by = who.and_then(|who| {
        let nick = Nick::from(who.split('!').next()?);

        let user = if let Some(user) = users.iter().find(|user| user.nickname() == nick) {
            user_context::view(
                selectable_text(who).style(theme::Text::Nickname(
                    user.color_seed(&config.buffer.nickname.color),
                    false,
                )),
                user.clone(),
            )
        } else {
            selectable_text(who).style(theme::Text::Server).into()
        };

        Some(row![
            selectable_text("set by ").style(theme::Text::Transparent),
            user,
            selectable_text(format!(" at {}", time?.to_rfc2822())).style(theme::Text::Transparent),
        ])
    });

    let content = column![selectable_text(text).style(theme::Text::Transparent)].push_maybe(set_by);

    let scrollable = scrollable(container(content).width(Length::Fill).padding(padding()))
        .style(theme::Scrollable::Hidden)
        .direction(scrollable::Direction::Vertical(
            scrollable::Properties::default()
                .alignment(scrollable::Alignment::Start)
                .width(5)
                .scroller_width(5),
        ));

    // Use double pass to limit layout to `max_lines` of text
    column![
        double_pass(
            container(column((0..max_lines).map(|_| "".into())))
                .width(Length::Fill)
                .padding(padding()),
            column![container(scrollable)].width(Length::Fill),
        ),
        vertical_space(1),
        horizontal_rule(1).style(theme::Rule::Default)
    ]
    .into()
}

fn padding() -> [u16; 2] {
    [0, 8]
}
