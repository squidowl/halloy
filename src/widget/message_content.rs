use data::appearance::theme::randomize_color;
use data::{message, Config};
use iced::widget::span;
use iced::widget::text::Span;
use iced::{border, Length};

use crate::{font, Theme};

use super::{selectable_rich_text, selectable_text, Element, Renderer};

pub fn message_content<'a, M: 'a>(
    content: &'a message::Content,
    theme: &'a Theme,
    on_link: impl Fn(message::Link) -> M + 'a,
    style: impl Fn(&Theme) -> selectable_text::Style + 'a,
    config: &Config,
) -> Element<'a, M> {
    message_content_impl::<(), M>(
        content,
        theme,
        on_link,
        style,
        Option::<(fn(&message::Link) -> _, fn(&message::Link, _, _) -> _)>::None,
        config,
    )
}

pub fn with_context<'a, T: Copy + 'a, M: 'a>(
    content: &'a message::Content,
    theme: &'a Theme,
    on_link: impl Fn(message::Link) -> M + 'a,
    style: impl Fn(&Theme) -> selectable_text::Style + 'a,
    link_entries: impl Fn(&message::Link) -> Vec<T> + 'a,
    entry: impl Fn(&message::Link, T, Length) -> Element<'a, M> + 'a,
    config: &Config,
) -> Element<'a, M> {
    message_content_impl(
        content,
        theme,
        on_link,
        style,
        Some((link_entries, entry)),
        config,
    )
}

#[allow(clippy::type_complexity)]
fn message_content_impl<'a, T: Copy + 'a, M: 'a>(
    content: &'a message::Content,
    theme: &'a Theme,
    on_link: impl Fn(message::Link) -> M + 'a,
    style: impl Fn(&Theme) -> selectable_text::Style + 'a,
    context_menu: Option<(
        impl Fn(&message::Link) -> Vec<T> + 'a,
        impl Fn(&message::Link, T, Length) -> Element<'a, M> + 'a,
    )>,
    config: &Config,
) -> Element<'a, M> {
    match content {
        data::message::Content::Plain(text) => selectable_text(text).style(style).into(),
        data::message::Content::Fragments(fragments) => {
            let mut text = selectable_rich_text::<M, message::Link, T, Theme, Renderer>(
                fragments
                    .iter()
                    .map(|fragment| match fragment {
                        data::message::Fragment::Text(s) => span(s),
                        data::message::Fragment::Channel(s) => span(s.as_str())
                            .color(theme.colors().buffer.url)
                            .link(message::Link::Channel(s.as_str().to_string())),
                        data::message::Fragment::User(user, text) => {
                            let color = theme.colors().buffer.nickname;
                            let seed = match &config.buffer.channel.message.nickname_color {
                                data::buffer::Color::Solid => None,
                                data::buffer::Color::Unique => Some(user.seed()),
                            };

                            let color = match seed {
                                Some(seed) => randomize_color(color, seed),
                                None => theme.colors().text.primary,
                            };

                            span(text)
                                .color(color)
                                .link(message::Link::User(user.clone()))
                        }
                        data::message::Fragment::Url(s) => span(s.as_str())
                            .color(theme.colors().buffer.url)
                            .link(message::Link::Url(s.as_str().to_string())),
                        data::message::Fragment::Formatted { text, formatting } => {
                            let mut span = span(text)
                                .color_maybe(
                                    formatting
                                        .fg
                                        .and_then(|color| color.into_iced(theme.colors())),
                                )
                                .background_maybe(
                                    formatting
                                        .bg
                                        .and_then(|color| color.into_iced(theme.colors())),
                                )
                                .underline(formatting.underline)
                                .strikethrough(formatting.strikethrough);

                            if formatting.monospace {
                                span = span
                                    .padding([0, 4])
                                    .color(theme.colors().buffer.code)
                                    .border(
                                        border::rounded(3)
                                            .color(theme.colors().general.border)
                                            .width(1),
                                    );
                            }

                            match (formatting.bold, formatting.italics) {
                                (true, true) => {
                                    span = span.font(font::MONO_BOLD_ITALICS.clone());
                                }
                                (true, false) => {
                                    span = span.font(font::MONO_BOLD.clone());
                                }
                                (false, true) => {
                                    span = span.font(font::MONO_ITALICS.clone());
                                }
                                (false, false) => {}
                            }

                            span
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .on_link(on_link)
            .style(style);

            if let Some((link_entries, view)) = context_menu {
                text = text.context_menu(link_entries, view);
            }

            text.into()
        }
        data::message::Content::Log(record) => {
            let mut spans: Vec<Span<'a, message::Link, _>> = vec![];

            spans.extend(
                config
                    .buffer
                    .format_timestamp(&record.timestamp)
                    .map(|ts| span(ts).color(theme.colors().buffer.timestamp)),
            );

            spans.extend([
                span(format!("{: <5}", record.level)).color(theme.colors().text.secondary),
                span(" "),
                span(&record.message),
            ]);

            selectable_rich_text::<M, message::Link, T, Theme, Renderer>(spans)
                .style(style)
                .into()
        }
    }
}
