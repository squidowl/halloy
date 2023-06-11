use iced::widget::text;

use crate::widget::Text;
use crate::{font, theme};

// Based off https://github.com/iced-rs/iced_aw/blob/main/src/graphics/icons/bootstrap.rs

pub fn globe<'a>() -> Text<'a> {
    to_text('\u{f3ef}')
}

pub fn _chat<'a>() -> Text<'a> {
    to_text('\u{f267}')
}

pub fn _person<'a>() -> Text<'a> {
    to_text('\u{f4e1}')
}

pub fn close<'a>() -> Text<'a> {
    to_text('\u{f659}')
}

pub fn maximize<'a>() -> Text<'a> {
    to_text('\u{f14a}')
}

pub fn restore<'a>() -> Text<'a> {
    to_text('\u{f149}')
}

pub fn people<'a>() -> Text<'a> {
    to_text('\u{f4db}')
}

fn to_text<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .line_height(1.1)
        .size(theme::ICON_SIZE)
        .font(font::ICON)
}
