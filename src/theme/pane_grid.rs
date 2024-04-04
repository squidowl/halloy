use iced::{
    widget::pane_grid::{Catalog, Highlight, Line, Style, StyleFn},
    Background, Border,
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
    Style {
        hovered_region: Highlight {
            background: Background::Color(theme.colors().action.high_alpha),
            border: Border {
                width: 1.0,
                color: theme.colors().action.base,
                radius: 4.0.into(),
            },
        },
        picked_split: Line {
            color: theme.colors().action.base,
            width: 4.0,
        },
        hovered_split: Line {
            color: theme.colors().action.base,
            width: 4.0,
        },
    }
}
