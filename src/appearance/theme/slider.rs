use iced::{
    widget::slider::{Catalog, Handle, HandleShape, Rail, Status, Style, StyleFn},
    Border,
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

pub fn primary(theme: &Theme, _status: Status) -> Style {
    Style {
        rail: Rail {
            backgrounds: (
                iced::Background::Color(theme.colors().general.background),
                iced::Background::Color(theme.colors().general.background),
            ),
            width: 8.0,
            border: Border {
                color: theme.colors().general.border,
                width: 0.5,
                radius: 4.0.into(),
            },
        },
        handle: Handle {
            shape: HandleShape::Circle { radius: 8.0 },
            background: iced::Background::Color(theme.colors().text.primary),
            border_width: 0.5,
            border_color: theme.colors().general.border,
        },
    }
}
