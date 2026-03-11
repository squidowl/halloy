use data::Config;
use data::isupport::CaseMap;
use iced::padding;
use iced::widget::{column, container};

use crate::widget::{self, Element};
use crate::{Theme, font, theme};

pub fn view<'a, Message: 'a>(
    typing: Option<String>,
    _config: &'a Config,
    theme: &'a Theme,
) -> Option<Element<'a, Message>> {
    let typing: Element<'a, Message> = match typing {
        Some(text) => container(
            widget::text(text)
                .style(theme::text::secondary)
                .font_maybe(theme::font_style::secondary(theme).map(font::get)),
        )
        .padding(padding::left(14).top(2).right(14))
        .align_y(iced::alignment::Vertical::Bottom)
        .style(theme::container::typing)
        .into(),
        None => column![].into(),
    };

    Some(typing)
}

pub fn typing_text(
    enabled: bool,
    supports_typing: bool,
    our_nick: Option<&str>,
    nicks: &[String],
    casemapping: CaseMap,
) -> Option<String> {
    if !enabled || !supports_typing {
        return None;
    }

    let filtered: Vec<_> = nicks
        .iter()
        .filter(|nick| {
            our_nick.is_none_or(|our| {
                casemapping.normalize(nick) != casemapping.normalize(our)
            })
        })
        .collect();

    match filtered.len() {
        0 => None,
        1 => Some(format!("{} is typing", filtered[0])),
        2 => Some(format!("{} and {} are typing", filtered[0], filtered[1])),
        _ => Some("Several people are typing".to_string()),
    }
}
