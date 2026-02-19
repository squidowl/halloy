use std::collections::{BTreeMap, BTreeSet};

use data::user::NickRef;
use iced::Padding;
pub use iced::widget::tooltip::Position;
use iced::widget::{Space, button, container, row, text};

use super::{Column, Element, Row};
use crate::theme;

pub fn reaction_row<'a, M, F1, F2>(
    message: &'a data::Message,
    our_nick: Option<NickRef<'a>>,
    on_press: Option<F1>,
    on_unreact: Option<F2>,
) -> Element<'a, M>
where
    M: 'a + Clone,
    F1: Fn(&'a str) -> M + 'a,
    F2: Fn(&'a str) -> M + 'a,
{
    // we need a container with deterministic order, so that the UI elements don't move around.
    let mut m: BTreeMap<&'a str, BTreeMap<&'a str, i16>> = BTreeMap::new();
    for r in message.reactions.iter() {
        let reactions_for_sender = m
            .entry(&r.text)
            .or_default()
            .entry(r.sender.as_str())
            .or_default();
        if r.unreact {
            *reactions_for_sender -= 1;
        } else {
            *reactions_for_sender += 1;
        }
    }
    let m: BTreeMap<&'a str, BTreeSet<&'a str>> = m
        .into_iter()
        .map(|(react, nicks)| {
            (
                react,
                nicks
                    .into_iter()
                    .filter_map(
                        |(nick, count)| {
                            if count >= 1 { Some(nick) } else { None }
                        },
                    )
                    .collect::<BTreeSet<&str>>(),
            )
        })
        .filter(|(_, set)| !set.is_empty())
        .collect();

    let row = Row::from_iter(m.iter().map(|(reaction_text, nicks)| {
        // we reacted to this message already
        let selected =
            our_nick.is_some_and(|nick| nicks.contains(&nick.as_str()));
        let on_press = if selected {
            on_unreact.as_ref().map(|f| f(reaction_text))
        } else {
            on_press.as_ref().map(|f| f(reaction_text))
        };
        let react_count = nicks.len();
        let mut button_content: Element<'a, M> = text(*reaction_text)
            .shaping(text::Shaping::Advanced)
            .style(theme::text::primary)
            .into();
        if react_count >= 2 {
            button_content = row![
                button_content,
                Space::new().width(4),
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
