use iced::widget::svg::{Catalog, Status, Style, StyleFn};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(none)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

pub fn none(_theme: &Theme, _status: Status) -> Style {
    Style { color: None }
}

pub fn primary(theme: &Theme, _status: Status) -> Style {
    Style {
        color: Some(theme.styles().text.primary.color),
    }
}

pub fn unread_indicator(theme: &Theme, _status: Status) -> Style {
    Style {
        color: Some(theme.styles().general.unread_indicator),
    }
}

pub fn highlight_indicator(theme: &Theme, _status: Status) -> Style {
    Style {
        color: theme
            .styles()
            .general
            .highlight_indicator
            .or(Some(theme.styles().general.unread_indicator)),
    }
}

pub fn error(theme: &Theme, _status: Status) -> Style {
    Style {
        color: Some(theme.styles().text.error.color),
    }
}
