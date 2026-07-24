use data::config;
use iced::widget::text::LineHeight;
use iced::widget::{svg, text};

use crate::widget::Text;
use crate::widget::text_color_svg::{TextColorSvg, text_color_svg};
use crate::{Theme, font, theme};

pub fn dot<'a>() -> Text<'a> {
    to_text('\u{F111}')
}

pub fn error<'a>() -> Text<'a> {
    to_text('\u{E80D}')
}

pub fn connected<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-globe.svg").as_slice(),
    ))
}

pub fn connecting<'a>() -> TextColorSvg<'a, Theme> {
    let fontawesome_plug =
        include_bytes!("../assets/fontello/fontawesome-plug.svg").to_vec();

    text_color_svg(svg::Handle::from_memory(fontawesome_plug))
}

// If attempting to connect and not successful
pub fn disconnected<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-cancel.svg").as_slice(),
    ))
}

// If not attempting to connect
pub fn not_connected<'a>() -> TextColorSvg<'a, Theme> {
    let elusive_error_alt =
        include_bytes!("../assets/fontello/elusive-error-alt.svg").to_vec();

    text_color_svg(svg::Handle::from_memory(elusive_error_alt))
}

pub fn link<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-link.svg").as_slice(),
    ))
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

pub fn search<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-search.svg").as_slice(),
    ))
}

pub fn checkmark<'a>() -> Text<'a> {
    to_text('\u{E806}')
}

pub fn file_transfer<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-download.svg").as_slice(),
    ))
}

pub fn refresh<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-arrows-ccw.svg").as_slice(),
    ))
}

pub fn megaphone<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-megaphone.svg").as_slice(),
    ))
}

pub fn theme_editor<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-palette.svg").as_slice(),
    ))
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

pub fn logs<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-bucket.svg").as_slice(),
    ))
}

pub fn menu<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/typicons-menu.svg").as_slice(),
    ))
}

pub fn documentation<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-book.svg").as_slice(),
    ))
}

pub fn highlights<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-bell.svg").as_slice(),
    ))
}

pub fn channel_monitor<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-desktop.svg").as_slice(),
    ))
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

pub fn config<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-file-code.svg")
            .as_slice(),
    ))
}

pub fn open<'a>() -> Text<'a> {
    to_text('\u{F115}')
}

pub fn star<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-star.svg").as_slice(),
    ))
}

pub fn certificate<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-certificate.svg")
            .as_slice(),
    ))
}

pub fn circle<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-circle.svg").as_slice(),
    ))
}

pub fn circle_empty<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-circle-empty.svg")
            .as_slice(),
    ))
}

pub fn dot_circled<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-dot-circled.svg")
            .as_slice(),
    ))
}

pub fn asterisk<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-asterisk.svg")
            .as_slice(),
    ))
}

pub fn speaker<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-sound.svg").as_slice(),
    ))
}

pub fn lightbulb<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-lightbulb.svg")
            .as_slice(),
    ))
}

pub fn quit<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/mfg-labs-logout.svg").as_slice(),
    ))
}

pub fn channel_discovery<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-users.svg").as_slice(),
    ))
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

pub fn not_sent<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/modern-pictograms-attention.svg")
            .as_slice(),
    ))
}

pub fn eraser<'a>() -> Text<'a> {
    to_text('\u{F12D}')
}

pub fn about<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-info-circled.svg").as_slice(),
    ))
}

pub fn log_indicator<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/entypo-cancel.svg").as_slice(),
    ))
}

pub fn show<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-eye.svg").as_slice(),
    ))
}

pub fn hide<'a>() -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/fontello/fontawesome-eye-off.svg").as_slice(),
    ))
}

pub fn spinner<'a>(angle: f32) -> TextColorSvg<'a, Theme> {
    text_color_svg(svg::Handle::from_memory(
        include_bytes!("../assets/spinner.svg").as_slice(),
    ))
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

pub fn from_icon<'a>(
    icon: config::sidebar::Icon,
) -> Option<TextColorSvg<'a, Theme>> {
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
