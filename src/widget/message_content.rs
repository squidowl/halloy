use data::appearance::theme::{FontStyle, randomize_color};
use data::{Config, isupport, message, target};
use iced::widget::span;
use iced::widget::text::Span;
use iced::{Color, Length, border};

use super::{Element, Renderer, selectable_rich_text, selectable_text};
use crate::{Theme, font, theme};

pub fn message_content<'a, M: 'a>(
    content: &'a message::Content,
    chantypes: &[char],
    casemapping: isupport::CaseMap,
    theme: &'a Theme,
    on_link: impl Fn(message::Link) -> M + 'a,
    default_link: Option<message::Link>,
    style: impl Fn(&Theme) -> selectable_text::Style + 'a,
    font_style: impl Fn(&Theme) -> Option<FontStyle>,
    color_transformation: Option<impl Fn(Color) -> Color>,
    config: &Config,
) -> Element<'a, M> {
    message_content_impl::<(), M>(
        content,
        chantypes,
        casemapping,
        theme,
        on_link,
        default_link,
        style,
        font_style,
        color_transformation,
        Option::<(fn(&message::Link) -> _, fn(&message::Link, _, _) -> _)>::None,
        config,
    )
}

pub fn with_context<'a, T: Copy + 'a, M: 'a>(
    content: &'a message::Content,
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
    config: &Config,
) -> Element<'a, M> {
    message_content_impl(
        content,
        chantypes,
        casemapping,
        theme,
        on_link,
        default_link,
        style,
        font_style,
        color_transformation,
        Some((link_entries, entry)),
        config,
    )
}

#[allow(clippy::type_complexity)]
fn message_content_impl<'a, T: Copy + 'a, M: 'a>(
    content: &'a message::Content,
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
    config: &Config,
) -> Element<'a, M> {
    match content {
        data::message::Content::Plain(text) => selectable_text(text)
            .font_maybe(font_style(theme).map(font::get))
            .style(style)
            .into(),
        data::message::Content::Fragments(fragments) => {
            let mut text = selectable_rich_text::<
                M,
                message::Link,
                T,
                Theme,
                Renderer,
            >(
                fragments
                    .iter()
                    .map(|fragment| {
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
                            data::message::Fragment::Text(s) => span(s),
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
                                        target::Channel::from_str(
                                            s.as_str(),
                                            chantypes,
                                            casemapping,
                                        ),
                                    ))
                            }
                            data::message::Fragment::User(user, text) => {
                                let color =
                                    theme.styles().buffer.nickname.color;
                                let seed = match &config
                                    .buffer
                                    .channel
                                    .message
                                    .nickname_color
                                {
                                    data::buffer::Color::Solid => None,
                                    data::buffer::Color::Unique => {
                                        Some(user.seed())
                                    }
                                };

                                let color = match seed {
                                    Some(seed) => randomize_color(color, seed),
                                    None => theme.styles().text.primary.color,
                                };

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
                                    .link(message::Link::User(user.clone()))
                            }
                            data::message::Fragment::HighlightNick(
                                user,
                                text,
                            ) => {
                                let color =
                                    theme.styles().buffer.nickname.color;
                                let seed = match &config
                                    .buffer
                                    .channel
                                    .message
                                    .nickname_color
                                {
                                    data::buffer::Color::Solid => None,
                                    data::buffer::Color::Unique => {
                                        Some(user.seed())
                                    }
                                };

                                let color = match seed {
                                    Some(seed) => randomize_color(color, seed),
                                    None => theme.styles().text.primary.color,
                                };

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
                                    .link(message::Link::User(user.clone()))
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
                            data::message::Fragment::Url(s) => span(s.as_str())
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
                                .link(message::Link::Url(
                                    s.as_str().to_string(),
                                )),
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

                        if span.link.is_none()
                            && let Some(default_link) = &default_link
                        {
                            span.link(default_link.clone())
                        } else {
                            span
                        }
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
