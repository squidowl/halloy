use iced::widget::rule::{Catalog, FillMode, Style, StyleFn};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

pub fn primary(theme: &Theme) -> Style {
    Style {
        color: theme.styles().general.horizontal_rule,
        radius: 0.0.into(),
        fill_mode: FillMode::Full,
        snap: true,
    }
}

pub fn backlog(theme: &Theme) -> Style {
    Style {
        color: theme
            .styles()
            .buffer
            .backlog_rule
            .unwrap_or(theme.styles().general.horizontal_rule),
        radius: 0.0.into(),
        fill_mode: FillMode::Full,
        snap: true,
    }
}
