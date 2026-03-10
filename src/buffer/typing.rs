use data::isupport::CaseMap;

use crate::widget::Element;
use crate::{Theme, font, theme};

pub fn view<'a, Message: 'a>(
    typing: Option<String>,
    theme: &'a Theme,
) -> Option<Element<'a, Message>> {
    let text = typing?;

    Some(
        crate::widget::text(text)
            .style(theme::text::secondary)
            .font_maybe(theme::font_style::secondary(theme).map(font::get))
            .into(),
    )
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
