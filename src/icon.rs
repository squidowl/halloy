use iced::widget::text;

use crate::{
    font, theme,
    widget::{Container, Text},
};

// Based off https://github.com/iced-rs/iced_aw/blob/main/src/graphics/icons/bootstrap.rs

pub fn close<'a>() -> Text<'a> {
    to_text('\u{f659}')
}

pub fn people<'a>() -> Text<'a> {
    to_text('\u{f826}')
}

fn to_text<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .size(theme::TEXT_SIZE)
        .font(font::ICON)
}
