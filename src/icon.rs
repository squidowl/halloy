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

pub fn connected<'a>() -> Svg<'a, Theme> {
    let entypo_globe =
        include_bytes!("../assets/fontello/entypo-globe.svg").to_vec();

    svg(svg::Handle::from_memory(entypo_globe))
}

pub fn disconnected<'a>() -> Svg<'a, Theme> {
    let entypo_cancel =
        include_bytes!("../assets/fontello/entypo-cancel.svg").to_vec();

    svg(svg::Handle::from_memory(entypo_cancel))
}

pub fn link<'a>() -> Svg<'a, Theme> {
    let entypo_link =
        include_bytes!("../assets/fontello/entypo-link.svg").to_vec();

    svg(svg::Handle::from_memory(entypo_link))
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
    to_text('\u{E81E}')
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

pub fn star<'a>() -> Svg<'a, Theme> {
    let fontawesome_star =
        include_bytes!("../assets/fontello/fontawesome-star.svg").to_vec();

    svg(svg::Handle::from_memory(fontawesome_star))
}

pub fn certificate<'a>() -> Svg<'a, Theme> {
    let fontawesome_certificate =
        include_bytes!("../assets/fontello/fontawesome-certificate.svg")
            .to_vec();

    svg(svg::Handle::from_memory(fontawesome_certificate))
}

pub fn circle<'a>() -> Svg<'a, Theme> {
    let fontawesome_circle =
        include_bytes!("../assets/fontello/fontawesome-circle.svg").to_vec();

    svg(svg::Handle::from_memory(fontawesome_circle))
}

pub fn circle_empty<'a>() -> Svg<'a, Theme> {
    let fontawesome_circle_empty =
        include_bytes!("../assets/fontello/fontawesome-circle-empty.svg")
            .to_vec();

    svg(svg::Handle::from_memory(fontawesome_circle_empty))
}

pub fn dot_circled<'a>() -> Svg<'a, Theme> {
    let fontawesome_dot_circled =
        include_bytes!("../assets/fontello/fontawesome-dot-circled.svg")
            .to_vec();

    svg(svg::Handle::from_memory(fontawesome_dot_circled))
}

pub fn asterisk<'a>() -> Svg<'a, Theme> {
    let fontawesome_asterisk =
        include_bytes!("../assets/fontello/fontawesome-asterisk.svg").to_vec();

    svg(svg::Handle::from_memory(fontawesome_asterisk))
}

pub fn speaker<'a>() -> Svg<'a, Theme> {
    let entypo_sound =
        include_bytes!("../assets/fontello/entypo-sound.svg").to_vec();

    svg(svg::Handle::from_memory(entypo_sound))
}

pub fn lightbulb<'a>() -> Svg<'a, Theme> {
    let fontawesome_lightbulb =
        include_bytes!("../assets/fontello/fontawesome-lightbulb.svg").to_vec();

    svg(svg::Handle::from_memory(fontawesome_lightbulb))
}

pub fn quit<'a>() -> Text<'a> {
    to_text('\u{F02D}')
}

pub fn channel_discovery<'a>() -> Text<'a> {
    to_text('\u{E81D}')
}

pub fn plus<'a>() -> Text<'a> {
    to_text('\u{E820}')
}

pub fn lock<'a>() -> Text<'a> {
    to_text('\u{E821}')
}

pub fn reply<'a>() -> Text<'a> {
    to_text('\u{E81B}')
}

pub fn not_sent<'a>() -> Svg<'a, Theme> {
    let fontawesome_attention =
        include_bytes!("../assets/fontello/fontawesome-attention.svg").to_vec();

    svg(svg::Handle::from_memory(fontawesome_attention))
}

pub fn spinner<'a>(angle: f32) -> Svg<'a, Theme> {
    let bytes = include_bytes!("../assets/spinner.svg").to_vec();

    svg(svg::Handle::from_memory(bytes))
        .width(15)
        .height(15)
        .rotation(iced::Radians(angle))
}

fn to_text<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .line_height(LineHeight::Relative(1.0))
        .size(theme::ICON_SIZE)
        .font(*font::ICON)
}

pub fn from_icon<'a>(icon: config::sidebar::Icon) -> Option<Svg<'a, Theme>> {
    match icon {
        config::sidebar::Icon::Dot => Some(circle()),
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
