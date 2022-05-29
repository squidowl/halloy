use data::theme::Theme;
use iced::{
    button::{Style, StyleSheet},
    Background, Color,
};

pub fn primary<'a>(theme: &'a Theme) -> Box<dyn StyleSheet + 'a> {
    Box::new(Primary(theme))
}

pub fn secondary<'a>(theme: &'a Theme) -> Box<dyn StyleSheet + 'a> {
    Box::new(Secondary(theme))
}

pub fn destruction<'a>(theme: &'a Theme) -> Box<dyn StyleSheet + 'a> {
    Box::new(Destruction(theme))
}

pub struct Primary<'a>(&'a Theme);

impl<'a> StyleSheet for Primary<'a> {
    fn active(&self) -> Style {
        Style {
            text_color: self.0.text.into(),
            border_width: 1.0,
            border_color: Color {
                a: 0.2,
                ..self.0.primary.darken().into()
            },
            ..Style::default()
        }
    }

    fn hovered(&self) -> Style {
        Style {
            text_color: self.0.primary.into(),
            background: Some(Background::Color(Color {
                a: 0.2,
                ..self.0.primary.into()
            })),
            border_width: 1.0,
            border_color: self.0.primary.darken().into(),
            ..Style::default()
        }
    }

    fn pressed(&self) -> Style {
        Style {
            background: Some(Background::Color(Color {
                a: 0.15,
                ..self.0.primary.into()
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
            text_color: self.0.text.into(),
            border_width: 1.0,
            border_color: Color {
                a: 0.2,
                ..self.0.error.darken().into()
            },
            ..Style::default()
        }
    }

    fn hovered(&self) -> Style {
        Style {
            text_color: self.0.error.into(),
            background: Some(Background::Color(Color {
                a: 0.2,
                ..self.0.error.into()
            })),
            border_width: 1.0,
            border_color: self.0.error.darken().into(),
            ..Style::default()
        }
    }

    fn pressed(&self) -> Style {
        Style {
            background: Some(Background::Color(Color {
                a: 0.15,
                ..self.0.error.into()
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

pub struct Secondary<'a>(&'a Theme);

impl<'a> StyleSheet for Secondary<'a> {
    fn active(&self) -> Style {
        Style {
            text_color: self.0.text.into(),
            ..Style::default()
        }
    }

    fn hovered(&self) -> Style {
        Style {
            text_color: self.0.primary.into(),
            background: Some(Background::Color(Color {
                a: 0.2,
                ..self.0.primary.into()
            })),
            border_width: 1.0,
            border_color: self.0.primary.darken().into(),
            ..Style::default()
        }
    }

    fn pressed(&self) -> Style {
        Style {
            background: Some(Background::Color(Color {
                a: 0.15,
                ..self.0.primary.into()
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
