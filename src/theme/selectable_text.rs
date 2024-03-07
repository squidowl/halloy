use data::message;

use crate::widget::selectable_text::{Appearance, DefaultStyle};

use super::{text, Theme};

impl DefaultStyle for Theme {
    fn default_style(&self) -> Appearance {
        Appearance {
            color: None,
            selection_color: self.colors().accent.high_alpha,
        }
    }
}

pub fn transparent(theme: &Theme) -> Appearance {
    let color = text::transparent(theme).color;

    Appearance {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn info(theme: &Theme) -> Appearance {
    let color = text::info(theme).color;

    Appearance {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn accent(theme: &Theme) -> Appearance {
    let color = text::accent(theme).color;

    Appearance {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn nickname(theme: &Theme, seed: Option<String>, transparent: bool) -> Appearance {
    let color = text::nickname(theme, seed, transparent).color;

    Appearance {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn status(theme: &Theme, status: message::source::Status) -> Appearance {
    let color = match status {
        message::source::Status::Success => text::success(theme).color,
        message::source::Status::Error => text::error(theme).color,
    };

    Appearance {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}
