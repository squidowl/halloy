use std::collections::{BTreeMap, BTreeSet};

use data::{message::Reaction, user::NickRef};
pub use iced::widget::tooltip::Position;
use iced::{
    Padding,
    widget::{Space, button, container, row, text},
};

use super::{Column, Element, Row};
use crate::theme;

pub fn reaction_row<'a, M>(
    _message: &'a data::Message,
    our_nick: Option<NickRef<'a>>,
    reactions: &'a [Reaction],
    on_press: impl Fn(&'a str) -> M + 'a,
) -> Element<'a, M>
where
    M: 'a + Clone,
{
    // we need containers with deterministic order, so that the UI elements don't move around.
    let mut map: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for r in reactions {
        map.entry(&r.text).or_default().insert(r.sender.as_ref());
    }

    let row = Row::from_iter(map.iter().map(|(reaction_text, nicks)| {
        // we reacted to this message already
        let selected =
            our_nick.is_some_and(|nick| nicks.contains(&nick.as_ref()));
        let on_press = if selected {
            // TODO(pounce) add unreaction.
            // currently useless as we do not support unreaction/redaction
            None
        } else {
            Some(on_press(reaction_text))
        };
        let react_count = nicks.len();
        let mut button_content: Element<'a, M> = text(*reaction_text)
            .shaping(text::Shaping::Advanced)
            .style(theme::text::primary)
            .into();
        if react_count >= 2 {
            button_content = row![
                button_content,
                Space::with_width(4),
                text(react_count.to_string()).style(theme::text::primary)
            ]
            .into();
        }
        let content: Element<'a, _> = button(button_content)
            .on_press_maybe(on_press)
            .style(move |theme, status| {
                theme::button::secondary(theme, status, selected)
            })
            .into();
        iced::widget::tooltip(
            content,
            container(Column::from_iter(nicks.iter().map(|nick| {
                text(*nick)
                    .shaping(text::Shaping::Advanced)
                    .style(theme::text::secondary)
                    .into()
            })))
            .style(theme::container::tooltip)
            .padding(8),
            Position::Bottom,
        )
        .into()
    }))
    .spacing(4)
    .wrap();
    // we want some spacing below to make sure that emojis look associated with the previous message
    container(row).padding(Padding::ZERO.bottom(8)).into()
}
