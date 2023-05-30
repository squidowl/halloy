use iced::widget::text;

use crate::{font, theme, widget::Text};

// Based off https://github.com/iced-rs/iced_aw/blob/main/src/graphics/icons/bootstrap.rs

pub fn close<'a>() -> Text<'a> {
    to_text('\u{f659}')
}

pub fn maximize<'a>() -> Text<'a> {
    to_text('\u{f14a}')
}

pub fn minimize<'a>() -> Text<'a> {
    to_text('\u{f149}')
}

pub fn people<'a>() -> Text<'a> {
    to_text('\u{f826}')
}

fn to_text<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .size(theme::TEXT_SIZE)
        .font(font::ICON)
}
