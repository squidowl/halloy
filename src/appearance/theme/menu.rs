pub use iced::widget::overlay::menu::Style;
use iced::widget::overlay::menu::{Catalog, StyleFn};
use iced::{Background, Border, Shadow};

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
    let buttons = theme.styles().buttons;
    let general = theme.styles().general;
    let text = theme.styles().text;

    Style {
        text_color: text.primary.color,
        background: Background::Color(general.background),
        border: Border {
            width: 1.0,
            radius: 4.0.into(),
            color: general.border,
        },
        selected_text_color: text.primary.color,
        selected_background: Background::Color(
            buttons.primary.background_hover,
        ),
        shadow: Shadow::default(),
    }
}
