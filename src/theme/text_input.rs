use iced::{
    widget::text_input::{Catalog, Status, Style, StyleFn},
    Background, Border, Color,
};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

pub fn primary(theme: &Theme, status: Status) -> Style {
    let active = Style {
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
        Status::Disabled => Style {
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

pub fn error(theme: &Theme, status: Status) -> Style {
    let primary = primary(theme, status);

    match status {
        Status::Active | Status::Hovered | Status::Focused => Style {
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
