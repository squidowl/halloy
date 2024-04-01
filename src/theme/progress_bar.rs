use iced::widget::progress_bar::{Appearance, DefaultStyle};

use super::Theme;

impl DefaultStyle for Theme {
    fn default_style(&self) -> Appearance {
        primary(self)
    }
}

pub fn primary(theme: &Theme) -> Appearance {
    Appearance {
        background: iced::Background::Color(theme.colors().background.darker),
        bar: iced::Background::Color(theme.colors().accent.low_alpha),
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
    }
}