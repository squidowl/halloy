use data::appearance::theme::{FontStyle, nickname_color};
use data::config::display::nickname::Metadata;
use data::target::Query;
use data::{Config, Server, User, isupport, message, metadata, target};
use iced::widget::text::Span;
use iced::widget::{button, span};
use iced::{Color, Length, border};
use unicode_segmentation::UnicodeSegmentation;

use super::{Element, Renderer, selectable_rich_text, selectable_text};
use crate::{Theme, font, theme};

pub fn message_content<'a, M: 'a + std::clone::Clone>(
    content: &'a message::Content,
    hidden_fragments: &[usize],
    server: &'a Server,
    registry: &'a dyn metadata::Registry,
    chantypes: &[char],
    casemapping: isupport::CaseMap,
    theme: &'a Theme,
    on_link: impl Fn(message::Link) -> M + 'a,
    default_link: Option<message::Link>,
    style: impl Fn(&Theme) -> selectable_text::Style + 'a,
    font_style: impl Fn(&Theme) -> Option<FontStyle>,
    color_transformation: Option<impl Fn(Color) -> Color>,
    nick_prefix_to_strip: Option<&str>,
    config: &Config,
) -> Element<'a, M> {
    message_content_impl::<(), M>(
        content,
        hidden_fragments,
        server,
        registry,
        chantypes,
        casemapping,
        theme,
        on_link,
        default_link,
        style,
        font_style,
        color_transformation,
        Option::<(fn(&message::Link) -> _, fn(&message::Link, _, _) -> _)>::None,
        nick_prefix_to_strip,
        config,
    )
}

pub fn with_context<'a, T: Copy + 'a, M: 'a + std::clone::Clone>(
    content: &'a message::Content,
    hidden_fragments: &[usize],
    server: &'a Server,
    registry: &'a dyn metadata::Registry,
    chantypes: &[char],
    casemapping: isupport::CaseMap,
    theme: &'a Theme,
    on_link: impl Fn(message::Link) -> M + 'a,
    default_link: Option<message::Link>,
    style: impl Fn(&Theme) -> selectable_text::Style + 'a,
    font_style: impl Fn(&Theme) -> Option<FontStyle>,
    color_transformation: Option<impl Fn(Color) -> Color>,
    link_entries: impl Fn(&message::Link) -> Vec<T> + 'a,
    entry: impl Fn(&message::Link, T, Length) -> Element<'a, M> + 'a,
    nick_prefix_to_strip: Option<&str>,
    config: &Config,
) -> Element<'a, M> {
    message_content_impl(
        content,
        hidden_fragments,
        server,
        registry,
        chantypes,
        casemapping,
        theme,
        on_link,
        default_link,
        style,
        font_style,
        color_transformation,
        Some((link_entries, entry)),
        nick_prefix_to_strip,
        config,
    )
}

#[allow(clippy::type_complexity)]
fn message_content_impl<'a, T: Copy + 'a, M: 'a + std::clone::Clone>(
    content: &'a message::Content,
    hidden_fragments: &[usize],
    server: &'a Server,
    registry: &'a dyn metadata::Registry,
    chantypes: &[char],
    casemapping: isupport::CaseMap,
    theme: &'a Theme,
    on_link: impl Fn(message::Link) -> M + 'a,
    default_link: Option<message::Link>,
    style: impl Fn(&Theme) -> selectable_text::Style + 'a,
    font_style: impl Fn(&Theme) -> Option<FontStyle>,
    color_transformation: Option<impl Fn(Color) -> Color>,
    context_menu: Option<(
        impl Fn(&message::Link) -> Vec<T> + 'a,
        impl Fn(&message::Link, T, Length) -> Element<'a, M> + 'a,
    )>,
    nick_prefix_to_strip: Option<&str>,
    config: &Config,
) -> Element<'a, M> {
    let color_from_user = |user: &User| -> Color {
        config
            .display
            .nickname
            .contains(&Metadata::Color)
            .then_some(registry)
            .and_then(|registry| registry.color(&Query::from(user)))
            .map_or(
                nickname_color(
                    theme.styles().buffer.nickname.color,
                    &config.buffer.nickname.color,
                    Some(user.seed()),
                ),
                |color| {
                    config.display.adapt_metadata_colors.adapt(
                        color,
                        theme.styles().buffer.nickname.color,
                        theme.styles().buffer.background,
                    )
                },
            )
    };

    match content {
        data::message::Content::Plain(text) => {
            let display_text: &str = nick_prefix_to_strip
                .and_then(|nick| strip_leading_nick(text.as_str(), nick))
                .filter(|s| !s.is_empty())
                .unwrap_or(text.as_str());

            let selectable_text = if let Some(only_emojis_size) =
                config.font.only_emojis_size
                && UnicodeSegmentation::graphemes(display_text, true)
                    .all(|grapheme| emojis::get(grapheme).is_some())
            {
                selectable_text(display_text)
                    .font_maybe(font_style(theme).map(font::get))
                    .size(f32::from(only_emojis_size))
                    .style(style)
            } else {
                selectable_text(display_text)
                    .font_maybe(font_style(theme).map(font::get))
                    .style(style)
            };

            if let Some(default_link) = default_link {
                button(selectable_text)
                    .style(theme::button::bare)
                    .padding(0)
                    .on_press(on_link(default_link))
                    .into()
            } else {
                selectable_text.into()
            }
        }
        data::message::Content::Fragments(fragments) => {
            let (prefix_skip_until, prefix_text_override) =
                nick_prefix_to_strip.map_or((0, None), |nick| {
                    leading_nick_offsets(fragments, nick)
                });
            let mut text = selectable_rich_text::<
                M,
                message::Link,
                T,
                Theme,
                Renderer,
            >(
                fragments
                    .iter()
                    .enumerate()
                    .filter_map(|(index, fragment)| {
                        if hidden_fragments.contains(&index)
                            || index < prefix_skip_until
                        {
                            return None;
                        }

                        let transform_color = |color: Color| -> Color {
                            if let Some(color_transformation) =
                                &color_transformation
                            {
                                color_transformation(color)
                            } else {
                                color
                            }
                        };

                        let span = match fragment {
                            data::message::Fragment::Text(s) => {
                                let text = prefix_text_override
                                    .and_then(|(idx, t)| {
                                        (idx == index).then_some(t)
                                    })
                                    .unwrap_or(s.as_str());
                                span(text)
                            }
                            data::message::Fragment::Channel(s) => {
                                span(s.as_str())
                                    .font_maybe(
                                        theme
                                            .styles()
                                            .buffer
                                            .url
                                            .font_style
                                            .map(font::get),
                                    )
                                    .color(transform_color(
                                        theme.styles().buffer.url.color,
                                    ))
                                    .link(message::Link::Channel(
                                        server.clone(),
                                        target::Channel::from_str(
                                            s.as_str(),
                                            chantypes,
                                            casemapping,
                                        ),
                                    ))
                            }
                            data::message::Fragment::User(user, text) => {
                                let color = color_from_user(user);

                                span(text)
                                    .font_maybe(
                                        theme
                                            .styles()
                                            .buffer
                                            .nickname
                                            .font_style
                                            .map(font::get),
                                    )
                                    .color(transform_color(color))
                                    .link(message::Link::User(
                                        server.clone(),
                                        user.clone(),
                                    ))
                            }
                            data::message::Fragment::HighlightNick(
                                user,
                                text,
                            ) => {
                                let color = color_from_user(user);

                                span(text)
                                    .font_maybe(
                                        theme
                                            .styles()
                                            .buffer
                                            .nickname
                                            .font_style
                                            .map(font::get),
                                    )
                                    .color(transform_color(color))
                                    .background(theme.styles().buffer.highlight)
                                    .link(message::Link::User(
                                        server.clone(),
                                        user.clone(),
                                    ))
                            }
                            data::message::Fragment::HighlightMatch(text) => {
                                span(text.as_str())
                                    .font_maybe(
                                        theme
                                            .styles()
                                            .text
                                            .primary
                                            .font_style
                                            .map(font::get),
                                    )
                                    .color(transform_color(
                                        theme.styles().text.primary.color,
                                    ))
                                    .background(theme.styles().buffer.highlight)
                            }
                            data::message::Fragment::Url(u, s) => if config
                                .display
                                .decode_urls
                            {
                                span(data::url::display(u))
                            } else {
                                span(s.as_str())
                            }
                            .font_maybe(
                                theme
                                    .styles()
                                    .buffer
                                    .url
                                    .font_style
                                    .map(font::get),
                            )
                            .color(transform_color(
                                theme.styles().buffer.url.color,
                            ))
                            // Copy to clipboard in IDNA-compliant encoding.
                            .link(message::Link::Url(u.as_str().to_string())),
                            data::message::Fragment::Formatted {
                                text,
                                formatting,
                            } => {
                                let mut span = span(text)
                                    .color_maybe(
                                        formatting
                                            .fg
                                            .and_then(|color| {
                                                color.into_iced(theme.styles())
                                            })
                                            .map(transform_color),
                                    )
                                    .background_maybe(formatting.bg.and_then(
                                        |color| color.into_iced(theme.styles()),
                                    ))
                                    .underline(formatting.underline)
                                    .strikethrough(formatting.strikethrough);

                                let formatted_style = if formatting.monospace {
                                    span = span
                                        .padding([0, 4])
                                        .color(theme.styles().buffer.code.color)
                                        .border(
                                            border::rounded(3)
                                                .color(
                                                    theme
                                                        .styles()
                                                        .general
                                                        .border,
                                                )
                                                .width(1),
                                        );

                                    theme.styles().buffer.code.font_style
                                } else {
                                    font_style(theme)
                                }
                                .or(Some(FontStyle::new(
                                    formatting.bold,
                                    formatting.italics,
                                )));

                                span.font_maybe(formatted_style.map(font::get))
                            }
                            data::message::Fragment::Condensed {
                                text,
                                source,
                                ..
                            } => span(text.as_str())
                                .font_maybe(
                                    theme::font_style::server(
                                        theme,
                                        Some(source),
                                    )
                                    .map(font::get),
                                )
                                .color_maybe(
                                    theme::selectable_text::server(
                                        theme,
                                        Some(source),
                                    )
                                    .color
                                    .map(transform_color),
                                ),
                        };

                        Some(
                            if span.link.is_none()
                                && let Some(default_link) = &default_link
                            {
                                span.link(default_link.clone())
                            } else {
                                span
                            },
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .on_link(on_link)
            .font_maybe(font_style(theme).map(font::get))
            .style(style);

            if let Some((link_entries, view)) = context_menu {
                text = text.context_menu(link_entries, view);
            }

            text.into()
        }
        data::message::Content::Log(record) => {
            let spans: Vec<Span<'a, message::Link, _>> = vec![
                span(&record.message)
                    .font_maybe(font_style(theme).map(font::get)),
            ];

            selectable_rich_text::<M, message::Link, T, Theme, Renderer>(spans)
                .style(style)
                .into()
        }
    }
}

pub(crate) fn strip_leading_nick<'a>(
    text: &'a str,
    nick: &str,
) -> Option<&'a str> {
    let prefix_len = nick.len() + 2;
    if text.len() >= prefix_len
        && text[..nick.len()].eq_ignore_ascii_case(nick)
        && text[nick.len()..].starts_with(": ")
    {
        Some(&text[prefix_len..])
    } else {
        None
    }
}

fn is_nick_fragment(fragment: &data::message::Fragment, nick: &str) -> bool {
    match fragment {
        data::message::Fragment::User(_, text)
        | data::message::Fragment::HighlightNick(_, text) => {
            text.eq_ignore_ascii_case(nick)
        }
        _ => false,
    }
}

fn leading_nick_offsets<'a>(
    fragments: &'a [data::message::Fragment],
    nick: &str,
) -> (usize, Option<(usize, &'a str)>) {
    match fragments {
        [data::message::Fragment::Text(text), remaining @ ..] => {
            match strip_leading_nick(text.as_str(), nick) {
                Some(rest) if !rest.is_empty() => (0, Some((0, rest))),
                Some(_) if !remaining.is_empty() => (1, None),
                _ => (0, None),
            }
        }
        [first, data::message::Fragment::Text(text), remaining @ ..]
            if is_nick_fragment(first, nick) && text.starts_with(": ") =>
        {
            let rest = &text[2..];
            if !rest.is_empty() {
                (1, Some((1, rest)))
            } else if !remaining.is_empty() {
                (2, None)
            } else {
                (0, None)
            }
        }
        _ => (0, None),
    }
}
