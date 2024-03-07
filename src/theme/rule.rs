use iced::widget::rule::{Appearance, DefaultStyle, FillMode};

use super::Theme;

impl DefaultStyle for Theme {
    fn default_style(&self) -> Appearance {
        primary(self)
    }
}

pub fn primary(theme: &Theme) -> Appearance {
    Appearance {
        color: theme.colors().background.light,
        width: 1,
        radius: 0.0.into(),
        fill_mode: FillMode::Full,
    }
}