use iced::{text_input::Style, text_input::StyleSheet, Background, Color};

use crate::theme::Theme;

pub fn primary<'a>(theme: &'a Theme) -> Box<dyn StyleSheet + 'a> {
    Box::new(Primary(theme))
}

pub struct Primary<'a>(&'a Theme);

impl<'a> StyleSheet for Primary<'a> {
    fn active(&self) -> Style {
        Style {
            background: Background::Color(self.0.background.lighten()),
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }

    fn focused(&self) -> Style {
        Style {
            background: Background::Color(self.0.background.lighten()),
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }

    fn hovered(&self) -> Style {
        Style {
            border_color: self.0.primary.into(),
            ..self.active()
        }
    }

    fn selection_color(&self) -> Color {
        self.0.primary.into()
    }

    fn placeholder_color(&self) -> Color {
        self.0.text.darken_by(0.25)
    }

    fn value_color(&self) -> Color {
        self.0.text.into()
    }
}
