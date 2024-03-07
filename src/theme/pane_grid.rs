use iced::{
    widget::pane_grid::{Appearance, DefaultStyle, Highlight, Line},
    Background, Border,
};

use super::Theme;

impl DefaultStyle for Theme {
    fn default_style(&self) -> Appearance {
        primary(self)
    }
}

pub fn primary(theme: &Theme) -> Appearance {
    Appearance {
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
