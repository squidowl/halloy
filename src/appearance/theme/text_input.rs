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
        background: Background::Color(theme.colors().buffer.background_text_input),
        border: Border {
            radius: 4.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
            // XXX Not currently displayed in application.
        },
        icon: theme.colors().text.primary,
        placeholder: theme.colors().text.secondary,
        value: theme.colors().text.primary,
        selection: theme.colors().buffer.selection,
    };

    match status {
        Status::Active | Status::Hovered | Status::Focused => active,
        Status::Disabled => Style {
            background: Background::Color(theme.colors().buffer.background_text_input),
            placeholder: Color {
                a: 0.2,
                ..theme.colors().text.secondary
            },
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
                color: theme.colors().text.error,
            },
            ..primary
        },
        Status::Disabled => primary,
    }
}
