use super::Theme;
use data::theme::{alpha, randomize_color};
use iced::widget::text::{Appearance, DefaultStyle};

impl DefaultStyle for Theme {
    fn default_style(&self) -> Appearance {
        none(self)
    }
}

pub fn none(_theme: &Theme) -> Appearance {
    Appearance { color: None }
}

pub fn primary(theme: &Theme) -> Appearance {
    Appearance {
        color: Some(theme.colors().text.base),
    }
}

pub fn accent(theme: &Theme) -> Appearance {
    Appearance {
        color: Some(theme.colors().accent.base),
    }
}

pub fn info(theme: &Theme) -> Appearance {
    Appearance {
        color: Some(theme.colors().info.base),
    }
}

pub fn error(theme: &Theme) -> Appearance {
    Appearance {
        color: Some(theme.colors().error.base),
    }
}

pub fn success(theme: &Theme) -> Appearance {
    Appearance {
        color: Some(theme.colors().success.base),
    }
}

pub fn transparent(theme: &Theme) -> Appearance {
    Appearance {
        color: Some(theme.colors().text.low_alpha),
    }
}

pub fn nickname(theme: &Theme, seed: Option<String>, transparent: bool) -> Appearance {
    let original_color = theme.colors().action.base;
    let randomized_color = seed
        .as_deref()
        .map(|seed| randomize_color(original_color, seed))
        .unwrap_or_else(|| original_color);

    let color = if transparent {
        let dark_theme = theme.colors().is_dark_theme();
        alpha(randomized_color, if dark_theme { 0.2 } else { 0.4 })
    } else {
        randomized_color
    };

    Appearance { color: Some(color) }
}
