use iced::{pure::text, Font, Text};

use crate::style;

// Based off https://github.com/iced-rs/iced_aw/blob/main/src/graphics/icons/bootstrap.rs

pub const FONT: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../fonts/icons.ttf"),
};

pub fn close() -> Text {
    to_text('\u{f5ae}')
}

pub fn box_arrow_down() -> Text {
    to_text('\u{f1a7}')
}

pub fn box_arrow_right() -> Text {
    to_text('\u{f1b1}')
}

#[allow(dead_code)]
pub fn raw(unicode: char) -> Text {
    to_text(unicode)
}

fn to_text(unicode: char) -> Text {
    text(unicode.to_string()).size(style::TEXT_SIZE).font(FONT)
}
