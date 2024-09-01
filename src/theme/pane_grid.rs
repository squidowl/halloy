use iced::{
    widget::pane_grid::{Catalog, Highlight, Line, Style, StyleFn},
    Background, Border, Color,
};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> StyleFn<'a, Self> {
        Box::new(primary)
    }

    fn style(&self, class: &StyleFn<'_, Self>) -> Style {
        class(self)
    }
}

pub fn primary(theme: &Theme) -> Style {
    let general = theme.colors().general;

    Style {
        hovered_region: Highlight {
            background: Background::Color(Color {
                a: 0.2,
                ..general.border
            }),
            border: Border {
                width: 1.0,
                color: general.border,
                radius: 4.0.into(),
            },
        },
        picked_split: Line {
            color: general.border,
            width: 4.0,
        },
        hovered_split: Line {
            color: general.border,
            width: 4.0,
        },
    }
}
