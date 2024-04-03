pub use iced::widget::overlay::menu::Style;
use iced::{
    widget::overlay::menu::{Catalog, StyleFn},
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

pub fn combo_box(theme: &Theme) -> Style {
    Style {
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
