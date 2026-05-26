use data::config::buffer::AccessLevelFormat;
use data::user::ChannelUsers;
use data::{Config, message, metadata};
use iced::alignment;
use iced::widget::text::Wrapping;
use iced::widget::{row, text};

use crate::widget::user_display::UserDisplay;
use crate::widget::{Element, Marker, message_content, message_marker};
use crate::{Theme, font, icon, theme};

/// Generates an element like `↩ alice: hi bob`
pub fn reply_preview_content<'a, Message: 'a + std::clone::Clone>(
    reply: Option<&'a message::ReplyPreview>,
    highlight: bool,
    show_icon: bool,
    text_size: f32,
    channel_users: Option<&'a ChannelUsers>,
    registry: &'a dyn metadata::Registry,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let char_width = font::width_from_str("a", &config.font);

    // the message may not be loaded
    let Some(reply) = reply else {
        return text("Replied to an unknown message")
            .style(theme::text::secondary)
            .size(text_size)
            .into();
    };

    let mut row = if show_icon {
        row![
            icon::reply()
                .size(config.buffer.reply.icon_size)
                .style(theme::text::primary)
        ]
    } else {
        row![]
    }
    .spacing(char_width);

    if !reply.blocked {
        if reply.is_action {
            row = row.push(message_marker(
                Marker::Dot,
                None,
                config,
                |t: &Theme| {
                    let style = theme::selectable_text::action(t);
                    crate::widget::selectable_text::Style {
                        color: style.color.map(|c| {
                            data::appearance::theme::alpha_color(c, 0.75)
                        }),
                        ..style
                    }
                },
                None::<Message>,
            ));
        }

        if let Some(user) = reply.user.as_ref() {
            let user = channel_users
                .and_then(|users| users.resolve(user))
                .unwrap_or(user);
            let user_display = if reply.is_action {
                UserDisplay::new(
                    user,
                    AccessLevelFormat::None,
                    false,
                    registry,
                    &config.display.nickname,
                    None,
                    config.display.truncation_character,
                    None,
                    false,
                )
            } else {
                UserDisplay::new(
                    user,
                    config.buffer.nickname.show_access_levels,
                    config.buffer.nickname.show_bot_icon,
                    registry,
                    &config.display.nickname,
                    config.buffer.nickname.truncate,
                    config.display.truncation_character,
                    Some(&config.buffer.nickname.brackets),
                    false,
                )
            };
            row = row.push(user_display.into_element(
                user,
                false,
                false,
                None,
                Some(text_size),
                highlight,
                false,
                theme,
                config,
            ));
        }
    }

    let preview_text_str = reply.preview_text();
    let inline_reply_nick = config
        .buffer
        .reply
        .hide_redundant_nicks
        .then(|| {
            reply
                .in_reply_to
                .as_deref()
                .and_then(|p| p.user.as_ref())
                .map(|u| u.nickname().as_str().to_owned())
        })
        .flatten();
    let preview = inline_reply_nick
        .as_deref()
        .and_then(|nick| {
            message_content::strip_leading_nick(&preview_text_str, nick)
        })
        .filter(|s| !s.is_empty())
        .map(std::borrow::ToOwned::to_owned)
        .unwrap_or(preview_text_str);

    let preview_text: Element<_> = text(preview)
        .style(move |t: &Theme| {
            if reply.is_action {
                iced::widget::text::Style {
                    color: theme::text::action(t)
                        .color
                        .map(|c| data::appearance::theme::alpha_color(c, 0.75)),
                }
            } else {
                theme::text::secondary(t)
            }
        })
        .size(text_size)
        .wrapping(Wrapping::None)
        .ellipsis(text::Ellipsis::End)
        .font_maybe(
            reply
                .is_action
                .then(|| theme::font_style::action(theme).map(font::get))
                .flatten(),
        )
        .into();

    row.push(preview_text)
        .align_y(alignment::Vertical::Center)
        .into()
}
