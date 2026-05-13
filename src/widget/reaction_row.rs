use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use data::isupport::CaseMap;
use data::user::{ChannelUsers, Nick, NickRef, User};
use data::{Config, metadata};
use iced::widget::text::LineHeight;
pub use iced::widget::tooltip::Position;
use iced::widget::{Space, button, container, row};
use iced::{alignment, padding};
use unicode_segmentation::UnicodeSegmentation;

use super::{Element, Row};
use crate::widget::user_display::UserDisplay;
use crate::widget::{text, tooltip};
use crate::{Theme, icon, theme};

const REACTION_TOOLTIP_MAX_NAMES: usize = 10;

pub fn has_visible_reactions(message: &data::Message) -> bool {
    !visible_reactions(message).is_empty()
}

pub fn reaction_row<'a, M, F1, F2>(
    message: &'a data::Message,
    our_nick: Option<NickRef<'a>>,
    font_size: f32,
    only_emojis_size: Option<f32>,
    max_reaction_display: u32,
    on_react: Option<F1>,
    on_unreact: Option<F2>,
    on_open_picker: Option<M>,
    show_tooltips_for_buttons: bool,
    users: Option<&'a ChannelUsers>,
    casemapping: CaseMap,
    config: &'a Config,
    registry: &'a dyn metadata::Registry,
    theme: &'a Theme,
) -> Element<'a, M>
where
    M: 'a + Clone,
    F1: Fn(&'a str) -> M + 'a,
    F2: Fn(&'a str) -> M + 'a,
{
    let emoji_size = (font_size - 1.0).max(10.0);
    let count_size = (font_size - 1.0).max(10.0);
    let m = visible_reactions(message);

    let mut row = Row::from_iter(m.iter().map(|(reaction_text, nicks)| {
        // we reacted to this message already
        let selected =
            our_nick.is_some_and(|nick| nicks.contains(&nick.as_str()));
        let on_press = if selected {
            on_unreact.as_ref().map(|f| f(reaction_text))
        } else {
            on_react.as_ref().map(|f| f(reaction_text))
        };
        let react_count = nicks.len();
        let emoji =
            text(truncate_text(reaction_text, max_reaction_display as usize))
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
            .on_press_maybe(on_press)
            .style(move |theme, status| {
                theme::button::secondary(theme, status, selected)
            })
            .into();

        let hidden_count =
            nicks.len().saturating_sub(REACTION_TOOLTIP_MAX_NAMES);
        let displayed: Vec<_> =
            nicks.iter().take(REACTION_TOOLTIP_MAX_NAMES).collect();
        let total = displayed.len() + usize::from(hidden_count > 0);
        let mut tooltip_nicks: Vec<Element<'a, M>> = vec![];
        for (i, nick) in displayed.iter().enumerate() {
            let resolved = users.and_then(|u| {
                u.iter().find(|u| u.nickname().as_str() == **nick)
            });
            let fallback;
            let user: &User = if let Some(user) = resolved {
                user
            } else {
                fallback = User::from(Nick::from_string(
                    nick.to_string(),
                    casemapping,
                ));
                &fallback
            };
            let nick_element = UserDisplay::new(
                user,
                config.buffer.nickname.show_access_levels,
                config.buffer.nickname.show_bot_icon,
                registry,
                &config.display.nickname,
                None,
                config.display.truncation_character,
                None,
                false,
            )
            .into_element(
                user, false, false, None, None, false, false, theme, config,
            );

            // trailing separator attached to nicks so it never starts a new line alone
            let position = i + 1;
            let trailing = if position + 1 == total && hidden_count == 0 {
                Some(" and")
            } else if position < displayed.len() {
                Some(",")
            } else {
                None
            };

            tooltip_nicks.push(match trailing {
                Some(t) => {
                    row![nick_element, text(t).style(theme::text::secondary),]
                        .into()
                }
                None => nick_element,
            });

            // space between items (wrappable, so lines start on the next nick)
            if position < total {
                tooltip_nicks
                    .push(text(" ").style(theme::text::secondary).into());
            }
        }
        if hidden_count > 0 {
            tooltip_nicks.push(
                text(format!("and {hidden_count} others…"))
                    .shaping(iced::widget::text::Shaping::Advanced)
                    .style(theme::text::secondary)
                    .into(),
            );
        }
        let tooltip_content = Row::from_iter(tooltip_nicks)
            .align_y(alignment::Vertical::Center)
            .wrap();

        let tooltip_emoji_size = if let Some(only_emojis_size) =
            only_emojis_size
            && UnicodeSegmentation::graphemes(*reaction_text, true)
                .all(|grapheme| emojis::get(grapheme).is_some())
        {
            only_emojis_size
        } else {
            font_size
        };

        let tooltip_content = row![
            text(*reaction_text)
                .shaping(iced::widget::text::Shaping::Advanced)
                .size(tooltip_emoji_size)
                .line_height(LineHeight::Relative(1.0))
                .style(theme::text::primary),
            tooltip_content,
        ]
        .spacing(8.0)
        .align_y(alignment::Vertical::Center);

        iced::widget::tooltip(
            content,
            container(tooltip_content)
                .style(theme::container::tooltip)
                .padding(8)
                .max_width(300),
            Position::Bottom,
        )
        .into()
    }))
    .padding(padding::all(0).top(2))
    .spacing(2.0);

    if !m.is_empty()
        && let Some(on_open_picker) = on_open_picker
    {
        let open_picker = tooltip(
            button(
                icon::plus()
                    .size(emoji_size + 4.0)
                    .style(theme::text::primary),
            )
            .padding([2, 6])
            .style(|theme, status| {
                theme::button::secondary(theme, status, false)
            })
            .on_press(on_open_picker),
            show_tooltips_for_buttons.then_some("Add reaction"),
            Position::Bottom,
            theme,
        );

        row = row.push(open_picker);
    }

    let row = row.wrap();

    container(row).into()
}

fn visible_reactions<'a>(
    message: &'a data::Message,
) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
    // We need a deterministic order so that the UI elements don't move around.
    let mut reactions = BTreeMap::<&'a str, BTreeMap<&'a str, i16>>::new();

    for reaction in &message.reactions {
        let reactions_for_sender = reactions
            .entry(&reaction.text)
            .or_default()
            .entry(reaction.sender.as_str())
            .or_default();

        if reaction.unreact {
            *reactions_for_sender -= 1;
        } else {
            *reactions_for_sender += 1;
        }
    }

    reactions
        .into_iter()
        .map(|(text, senders)| {
            (
                text,
                senders
                    .into_iter()
                    .filter_map(|(sender, count)| {
                        (count >= 1).then_some(sender)
                    })
                    .collect::<BTreeSet<&str>>(),
            )
        })
        .filter(|(_, senders)| !senders.is_empty())
        .collect()
}

pub fn truncate_text<'a>(text: &'a str, max_chars: usize) -> Cow<'a, str> {
    if UnicodeSegmentation::graphemes(text, true).count() <= max_chars {
        return text.into();
    }

    let mut truncated = UnicodeSegmentation::graphemes(text, true)
        .take(max_chars)
        .collect::<String>();
    truncated.push('…');
    truncated.into()
}

#[cfg(test)]
mod tests {
    use super::truncate_text;

    #[test]
    fn keeps_short_reaction_text() {
        assert_eq!(truncate_text("hello", 5), "hello");
    }

    #[test]
    fn truncates_ascii_to_limit() {
        assert_eq!(truncate_text("hello world", 5), "hello…");
    }

    #[test]
    fn truncates_unicode_graphemes() {
        assert_eq!(truncate_text("cafe\u{301}", 4), "cafe\u{301}");
    }

    #[test]
    fn limit_one_keeps_first_grapheme_when_truncated() {
        assert_eq!(truncate_text("👍🏽👍🏽", 1), "👍🏽…");
    }

    #[test]
    fn does_not_split_zwj_emoji_clusters() {
        assert_eq!(truncate_text("👨‍👩‍👧‍👦x", 1), "👨‍👩‍👧‍👦…");
    }

    #[test]
    fn does_not_split_combining_mark_clusters() {
        assert_eq!(truncate_text("a\u{0301}b", 1), "a\u{0301}…");
    }
}
