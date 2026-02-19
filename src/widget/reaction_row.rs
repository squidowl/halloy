use std::collections::{BTreeMap, BTreeSet};

use data::user::NickRef;
use iced::alignment;
pub use iced::widget::tooltip::Position;
use iced::widget::{Space, button, container, row};

use super::{Column, Element, Row};
use crate::theme;
use crate::widget::text;

const REACTION_TOOLTIP_MAX_NAMES: usize = 10;

pub fn reaction_row<'a, M, F1, F2>(
    message: &'a data::Message,
    our_nick: Option<NickRef<'a>>,
    font_size: f32,
    on_react: Option<F1>,
    on_unreact: Option<F2>,
) -> Element<'a, M>
where
    M: 'a + Clone,
    F1: Fn(&'a str) -> M + 'a,
    F2: Fn(&'a str) -> M + 'a,
{
    let emoji_size = (font_size - 1.0).max(10.0);
    let count_size = (font_size - 1.0).max(10.0);
    let button_height = (emoji_size + 8.0).max(22.0);

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
            on_react.as_ref().map(|f| f(reaction_text))
        };
        let react_count = nicks.len();
        let emoji = text(*reaction_text)
            .shaping(iced::widget::text::Shaping::Advanced)
            .size(emoji_size)
            .style(theme::text::primary);
        let mut button_content = row![emoji];
        if react_count >= 2 {
            button_content = button_content.push(Space::new().width(4)).push(
                text(react_count.to_string())
                    .size(count_size)
                    .style(theme::text::primary),
            );
        }
        let button_content: Element<'a, M> =
            button_content.align_y(alignment::Vertical::Center).into();

        let content: Element<'a, M> = button(button_content)
            .padding([2, 6])
            .height(button_height)
            .on_press_maybe(on_press)
            .style(move |theme, status| {
                theme::button::secondary(theme, status, selected)
            })
            .into();

        let hidden_count =
            nicks.len().saturating_sub(REACTION_TOOLTIP_MAX_NAMES);
        let mut tooltip_content = Column::from_iter(
            nicks.iter().take(REACTION_TOOLTIP_MAX_NAMES).map(|nick| {
                text(*nick)
                    .shaping(iced::widget::text::Shaping::Advanced)
                    .style(theme::text::secondary)
                    .into()
            }),
        );
        if hidden_count > 0 {
            tooltip_content = tooltip_content.push(
                text(format!("and {hidden_count} others..."))
                    .shaping(iced::widget::text::Shaping::Advanced)
                    .style(theme::text::secondary),
            );
        }

        iced::widget::tooltip(
            content,
            container(tooltip_content)
                .style(theme::container::tooltip)
                .padding(8),
            Position::Bottom,
        )
        .into()
    }))
    .spacing(2.0)
    .wrap();

    container(row).into()
}
