use iced::{
    button::{Style, StyleSheet},
    Background, Color,
};

use crate::theme::Theme;

pub fn primary<'a>(theme: &'a Theme) -> Box<dyn StyleSheet + 'a> {
    Box::new(Primary(theme))
}

pub fn destruction<'a>(theme: &'a Theme) -> Box<dyn StyleSheet + 'a> {
    Box::new(Destruction(theme))
}

pub struct Primary<'a>(&'a Theme);

impl<'a> StyleSheet for Primary<'a> {
    fn active(&self) -> Style {
        Style {
            text_color: self.0.palette.text.into(),
            border_width: 1.0,
            border_color: Color {
                a: 0.2,
                ..self.0.palette.primary.darken()
            },
            ..Style::default()
        }
    }

    fn hovered(&self) -> Style {
        Style {
            text_color: self.0.palette.primary.into(),
            background: Some(Background::Color(Color {
                a: 0.2,
                ..self.0.palette.primary.into()
            })),
            border_width: 1.0,
            border_color: self.0.palette.primary.darken(),
            ..Style::default()
        }
    }

    fn pressed(&self) -> Style {
        Style {
            background: Some(Background::Color(Color {
                a: 0.15,
                ..self.0.palette.primary.into()
            })),
            ..self.active()
        }
    }

    fn disabled(&self) -> Style {
        let active = self.active();

        Style {
            text_color: Color {
                a: 0.2,
                ..active.text_color
            },
            border_color: Color {
                a: 0.2,
                ..active.border_color
            },
            ..active
        }
    }
}

pub struct Destruction<'a>(&'a Theme);

impl<'a> StyleSheet for Destruction<'a> {
    fn active(&self) -> Style {
        Style {
            text_color: self.0.palette.text.into(),
            border_width: 1.0,
            border_color: Color {
                a: 0.2,
                ..self.0.palette.error.darken()
            },
            ..Style::default()
        }
    }

    fn hovered(&self) -> Style {
        Style {
            text_color: self.0.palette.error.into(),
            background: Some(Background::Color(Color {
                a: 0.2,
                ..self.0.palette.error.into()
            })),
            border_width: 1.0,
            border_color: self.0.palette.error.darken(),
            ..Style::default()
        }
    }

    fn pressed(&self) -> Style {
        Style {
            background: Some(Background::Color(Color {
                a: 0.15,
                ..self.0.palette.error.into()
            })),
            ..self.active()
        }
    }

    fn disabled(&self) -> Style {
        let active = self.active();

        Style {
            text_color: Color {
                a: 0.2,
                ..active.text_color
            },
            border_color: Color {
                a: 0.2,
                ..active.border_color
            },
            ..active
        }
    }
}
