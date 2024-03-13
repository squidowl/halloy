use iced::{
    widget::text_input::{Appearance, DefaultStyle, Status},
    Background, Border, Color,
};

use super::Theme;

impl DefaultStyle for Theme {
    fn default_style(&self, status: Status) -> Appearance {
        primary(self, status)
    }
}

pub fn primary(theme: &Theme, status: Status) -> Appearance {
    let active = Appearance {
        background: Background::Color(theme.colors().background.darker),
        border: Border {
            radius: 4.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
            // XXX Not currently displayed in application.
        },
        icon: theme.colors().text.base,
        placeholder: theme.colors().text.low_alpha,
        value: theme.colors().text.base,
        selection: theme.colors().accent.high_alpha,
    };

    match status {
        Status::Active | Status::Hovered | Status::Focused => active,
        Status::Disabled => Appearance {
            background: Background::Color(theme.colors().background.low_alpha),
            border: Border {
                radius: 4.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
                // XXX Not currently displayed in application.
            },
            ..active
        },
    }
}

pub fn error(theme: &Theme, status: Status) -> Appearance {
    let primary = primary(theme, status);

    match status {
        Status::Active | Status::Hovered | Status::Focused => Appearance {
            border: Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.colors().error.base,
            },
            ..primary
        },
        Status::Disabled => primary,
    }
}
