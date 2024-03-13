pub use iced::widget::overlay::menu::Style;
use iced::{
    widget::overlay::menu::{Appearance, DefaultStyle},
    Background, Border,
};

use super::{scrollable, Theme};

impl DefaultStyle for Theme {
    fn default_style() -> Style<'static, Self> {
        Style {
            list: Box::new(primary),
            scrollable: Box::new(scrollable::primary),
        }
    }
}

pub fn primary(theme: &Theme) -> Appearance {
    Appearance {
        text_color: theme.colors().text.base,
        background: Background::Color(theme.colors().background.base),
        border: Border {
            width: 1.0,
            radius: 4.0.into(),
            color: theme.colors().action.base,
        },
        selected_text_color: theme.colors().text.high_alpha,
        selected_background: Background::Color(theme.colors().background.high_alpha),
    }
}

pub fn combo_box(theme: &Theme) -> Appearance {
    Appearance {
        text_color: theme.colors().text.base,
        background: Background::Color(theme.colors().background.base),
        border: Border {
            width: 1.0,
            radius: 4.0.into(),
            color: if theme.colors().is_dark_theme() {
                theme.colors().background.lighter
            } else {
                theme.colors().background.darker
            },
        },
        selected_text_color: theme.colors().text.base,
        selected_background: Background::Color(theme.colors().background.dark),
    }
}
