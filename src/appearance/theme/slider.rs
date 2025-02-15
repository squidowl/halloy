use iced::{
    widget::slider::{Catalog, Handle, HandleShape, Rail, Status, Style, StyleFn},
    Background, Border, Color,
};

use super::{container::general, Theme};

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
    Style {
        rail: Rail {
            backgrounds: (
                iced::Background::Color(theme.colors().general.background),
                iced::Background::Color(theme.colors().buffer.background),
            ),
            width: 12.0,
            border: Border {
                color: theme.colors().general.border,
                width: 1.0,
                radius: 4.0.into(),
            },
        },
        handle: Handle {
            shape: HandleShape::Circle { radius: 12.0 },
            background: iced::Background::Color(theme.colors().text.primary),
            border_width: 1.0,
            border_color: theme.colors().general.border,
        },
    }
}
