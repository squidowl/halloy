#![allow(dead_code)]
use data::message;
use iced::border;
use iced::widget::span;

use crate::{font, Theme};

pub use self::anchored_overlay::anchored_overlay;
pub use self::combo_box::combo_box;
pub use self::context_menu::context_menu;
pub use self::double_pass::double_pass;
pub use self::key_press::key_press;
pub use self::modal::modal;
pub use self::selectable_rich_text::selectable_rich_text;
pub use self::selectable_text::selectable_text;
pub use self::shortcut::shortcut;
pub use self::tooltip::tooltip;

pub mod anchored_overlay;
pub mod collection;
pub mod combo_box;
pub mod context_menu;
pub mod double_click;
pub mod double_pass;
pub mod hover;
pub mod key_press;
pub mod modal;
pub mod selectable_rich_text;
pub mod selectable_text;
pub mod shortcut;
pub mod tooltip;

pub type Renderer = iced::Renderer;
pub type Element<'a, Message> = iced::Element<'a, Message, Theme, Renderer>;
pub type Content<'a, Message> = iced::widget::pane_grid::Content<'a, Message, Theme, Renderer>;
pub type TitleBar<'a, Message> = iced::widget::pane_grid::TitleBar<'a, Message, Theme, Renderer>;
pub type Column<'a, Message> = iced::widget::Column<'a, Message, Theme, Renderer>;
pub type Row<'a, Message> = iced::widget::Row<'a, Message, Theme, Renderer>;
pub type Text<'a> = iced::widget::Text<'a, Theme, Renderer>;
pub type Container<'a, Message> = iced::widget::Container<'a, Message, Theme, Renderer>;
pub type Button<'a, Message> = iced::widget::Button<'a, Message, Theme>;

pub fn message_content<'a, M: 'a>(
    content: &'a message::Content,
    theme: &'a Theme,
    on_link: impl Fn(String) -> M + 'a,
    style: impl Fn(&Theme) -> selectable_text::Style + 'a,
) -> Element<'a, M> {
    match content {
        data::message::Content::Plain(text) => selectable_text(text).style(style).into(),
        data::message::Content::Fragments(fragments) => selectable_rich_text(
            fragments
                .iter()
                .map(|fragment| match fragment {
                    data::message::Fragment::Text(s) => span(s),
                    data::message::Fragment::Url(s) => span(s.as_str())
                        .color(theme.colors().action.base)
                        .link(s.as_str().to_string()),
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
                                .color(theme.colors().error.darker)
                                .background(theme.colors().background.lighter)
                                .border(border::rounded(3));
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
        .style(style)
        .into(),
    }
}
