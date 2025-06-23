use iced::widget::checkbox::{Catalog, Status, Style, StyleFn};
use iced::{Border, Color};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(
        &self,
        class: &Self::Class<'_>,
        status: iced::widget::checkbox::Status,
    ) -> Style {
        class(self, status)
    }
}

pub fn primary(theme: &Theme, status: Status) -> Style {
    let general = theme.colors().general;
    let text = theme.colors().text;

    match status {
        Status::Active { .. } => Style {
            background: iced::Background::Color(general.background),
            icon_color: text.primary,
            border: Border {
                color: general.border,
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: Some(text.primary),
        },
        Status::Hovered { .. } => Style {
            background: iced::Background::Color(general.background),
            icon_color: text.primary,
            border: Border {
                color: general.border,
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: Some(text.primary),
        },
        Status::Disabled { .. } => Style {
            background: iced::Background::Color(general.background),

            icon_color: Color {
                a: 0.2,
                ..text.primary
            },
            border: Border {
                color: Color::TRANSPARENT,
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: Some(Color {
                a: 0.2,
                ..text.primary
            }),
        },
    }
}
