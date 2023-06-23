use iced::widget::text;
use iced::widget::text::LineHeight;

use crate::widget::Text;
use crate::{font, theme};

// Based off https://github.com/iced-rs/iced_aw/blob/main/src/graphics/icons/bootstrap.rs

pub fn error<'a>() -> Text<'a> {
    to_text('\u{f33a}')
}

pub fn globe<'a>() -> Text<'a> {
    to_text('\u{f3ef}')
}

pub fn wifi_off<'a>() -> Text<'a> {
    to_text('\u{f61b}')
}

pub fn chat<'a>() -> Text<'a> {
    to_text('\u{f267}')
}

pub fn person<'a>() -> Text<'a> {
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
        .style(theme::Text::Primary)
        .line_height(LineHeight::Relative(1.1))
        .size(theme::ICON_SIZE)
        .font(font::ICON)
}
