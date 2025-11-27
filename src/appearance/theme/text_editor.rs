use iced::widget::text_editor::{Catalog, Status, Style, StyleFn};
use iced::{Background, Border, Color};

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
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
            // XXX Not currently displayed in application.
        },
        placeholder: theme.styles().text.secondary.color,
        value: theme.styles().text.primary.color,
        selection: theme.styles().buffer.selection,
    };

    match status {
        Status::Active | Status::Hovered | Status::Focused { .. } => active,
        Status::Disabled => Style {
            background: Background::Color(Color::TRANSPARENT),
            placeholder: Color {
                a: 0.2,
                ..theme.styles().text.secondary.color
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
        Status::Active | Status::Hovered | Status::Focused { .. } => Style {
            border: Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.styles().text.error.color,
            },
            ..primary
        },
        Status::Disabled => primary,
    }
}
