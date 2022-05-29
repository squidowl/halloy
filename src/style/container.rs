use data::theme::Theme;
use iced::{pure::widget::container, Background};

pub fn pane(theme: &Theme, is_focused: bool) -> Pane {
    Pane { theme, is_focused }
}

pub fn header(theme: &Theme) -> Header {
    Header { theme }
}

pub fn primary(theme: &Theme) -> Primary {
    Primary { theme }
}

pub struct Pane<'a> {
    theme: &'a Theme,
    is_focused: bool,
}

impl<'a> container::StyleSheet for Pane<'a> {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(self.theme.background.into())),
            border_width: 2.0,
            border_color: if self.is_focused {
                self.theme.background.lighten().into()
            } else {
                self.theme.background.into()
            },
            ..Default::default()
        }
    }
}

pub struct Header<'a> {
    theme: &'a Theme,
}

impl<'a> container::StyleSheet for Header<'a> {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(self.theme.background.lighten().into())),
            ..Default::default()
        }
    }
}

pub struct Primary<'a> {
    theme: &'a Theme,
}

impl<'a> container::StyleSheet for Primary<'a> {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(self.theme.background.darken().into())),
            text_color: Some(self.theme.text.into()),
            ..Default::default()
        }
    }
}
