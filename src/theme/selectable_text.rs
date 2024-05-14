use data::{message, user::NickColor};

use crate::widget::selectable_text::{Catalog, Style, StyleFn};

use super::{text, Theme};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme| Style {
            color: None,
            selection_color: theme.colors().accent.high_alpha,
        })
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

pub fn transparent(theme: &Theme) -> Style {
    let color = text::transparent(theme).color;

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn info(theme: &Theme) -> Style {
    let color = text::info(theme).color;

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn accent(theme: &Theme) -> Style {
    let color = text::accent(theme).color;

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn nickname(theme: &Theme, nick_color: NickColor, transparent: bool) -> Style {
    let color = text::nickname(theme, nick_color, transparent).color;

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn status(theme: &Theme, status: message::source::Status) -> Style {
    let color = match status {
        message::source::Status::Success => text::success(theme).color,
        message::source::Status::Error => text::error(theme).color,
    };

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}
