use data::config;
use iced::widget::text::LineHeight;
use iced::widget::{Svg, svg, text};

use crate::widget::Text;
use crate::{Theme, font, theme};

pub fn dot<'a>() -> Text<'a> {
    to_text('\u{F111}')
}

pub fn error<'a>() -> Text<'a> {
    to_text('\u{E80D}')
}

pub fn connected<'a>() -> Text<'a> {
    to_text('\u{E800}')
}

pub fn link() -> Text<'static> {
    to_text('\u{E814}')
}

pub fn cancel<'a>() -> Text<'a> {
    to_text('\u{E80F}')
}

pub fn maximize<'a>() -> Text<'a> {
    to_text('\u{E801}')
}

pub fn restore<'a>() -> Text<'a> {
    to_text('\u{E805}')
}

pub fn people<'a>() -> Text<'a> {
    to_text('\u{E804}')
}

pub fn topic<'a>() -> Text<'a> {
    to_text('\u{E803}')
}

pub fn search<'a>() -> Text<'a> {
    to_text('\u{E808}')
}

pub fn checkmark<'a>() -> Text<'a> {
    to_text('\u{E806}')
}

pub fn file_transfer<'a>() -> Text<'a> {
    to_text('\u{E802}')
}

pub fn refresh<'a>() -> Text<'a> {
    to_text('\u{E807}')
}

pub fn megaphone<'a>() -> Text<'a> {
    to_text('\u{E809}')
}

pub fn theme_editor<'a>() -> Text<'a> {
    to_text('\u{E80A}')
}

pub fn undo<'a>() -> Text<'a> {
    to_text('\u{E80B}')
}

pub fn copy<'a>() -> Text<'a> {
    to_text('\u{F0C5}')
}

pub fn popout<'a>() -> Text<'a> {
    to_text('\u{E80E}')
}

pub fn logs<'a>() -> Text<'a> {
    to_text('\u{E810}')
}

pub fn menu<'a>() -> Text<'a> {
    to_text('\u{F0C9}')
}

pub fn documentation<'a>() -> Text<'a> {
    to_text('\u{E812}')
}

pub fn highlights<'a>() -> Text<'a> {
    to_text('\u{E811}')
}

pub fn scroll_to_bottom<'a>() -> Text<'a> {
    to_text('\u{F103}')
}

pub fn share<'a>() -> Text<'a> {
    to_text('\u{E813}')
}

pub fn mark_as_read<'a>() -> Text<'a> {
    to_text('\u{E817}')
}

pub fn config<'a>() -> Text<'a> {
    to_text('\u{F1C9}')
}

pub fn star<'a>() -> Text<'a> {
    to_text('\u{E819}')
}

pub fn certificate<'a>() -> Text<'a> {
    to_text('\u{F0A3}')
}

pub fn circle_empty<'a>() -> Text<'a> {
    to_text('\u{F10C}')
}

pub fn dot_circled<'a>() -> Text<'a> {
    to_text('\u{F192}')
}

pub fn asterisk<'a>() -> Text<'a> {
    to_text('\u{E815}')
}

pub fn speaker<'a>() -> Text<'a> {
    to_text('\u{E818}')
}

pub fn lightbulb<'a>() -> Text<'a> {
    to_text('\u{F0EB}')
}

pub fn quit<'a>() -> Text<'a> {
    to_text('\u{F02D}')
}

pub fn not_sent<'a>() -> Svg<'a, Theme> {
    let fontawesome_attention_circled =
        include_bytes!("../assets/fontello/fontawesome-attention-circled.svg")
            .to_vec();

    svg(svg::Handle::from_memory(fontawesome_attention_circled))
}

fn to_text<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .line_height(LineHeight::Relative(1.0))
        .size(theme::ICON_SIZE)
        .font(font::ICON)
}

pub fn from_icon<'a>(icon: config::sidebar::Icon) -> Option<Text<'a>> {
    match icon {
        config::sidebar::Icon::Dot => Some(dot()),
        config::sidebar::Icon::DotCircled => Some(dot_circled()),
        config::sidebar::Icon::Certificate => Some(certificate()),
        config::sidebar::Icon::Asterisk => Some(asterisk()),
        config::sidebar::Icon::Speaker => Some(speaker()),
        config::sidebar::Icon::Lightbulb => Some(lightbulb()),
        config::sidebar::Icon::Star => Some(star()),
        config::sidebar::Icon::CircleEmpty => Some(circle_empty()),
        config::sidebar::Icon::None => None,
    }
}
