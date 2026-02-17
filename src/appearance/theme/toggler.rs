use iced::widget::toggler::{Catalog, Status, Style, StyleFn};
use iced::{Background, Color};

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
    let general = theme.styles().general;
    let text = theme.styles().text;
    let secondary = theme.styles().buttons.secondary;

    let (background, foreground) = match status {
        Status::Active { is_toggled } => {
            if is_toggled {
                (secondary.background_selected, text.primary.color)
            } else {
                (secondary.background, text.primary.color)
            }
        }
        Status::Hovered { is_toggled } => {
            if is_toggled {
                (secondary.background_selected_hover, text.primary.color)
            } else {
                (secondary.background_hover, text.primary.color)
            }
        }
        Status::Disabled { is_toggled } => {
            let background = if is_toggled {
                secondary.background_selected
            } else {
                secondary.background
            };

            (
                Color {
                    a: 0.2,
                    ..background
                },
                Color {
                    a: 0.2,
                    ..text.primary.color
                },
            )
        }
    };

    Style {
        background: Background::Color(background),
        background_border_width: 1.0,
        background_border_color: general.border,
        foreground: Background::Color(foreground),
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        text_color: Some(text.primary.color),
        border_radius: None,
        padding_ratio: 0.15,
    }
}
