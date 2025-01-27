use data::appearance::theme::{alpha_color, randomize_color};
use iced::{
    widget::text::{Catalog, Style, StyleFn},
    Color,
};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(none)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

pub fn none(_theme: &Theme) -> Style {
    Style { color: None }
}

pub fn primary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().text.primary),
    }
}

pub fn secondary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().text.secondary),
    }
}

pub fn tertiary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().text.tertiary),
    }
}

pub fn error(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().text.error),
    }
}

pub fn success(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().text.success),
    }
}

pub fn action(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().buffer.action),
    }
}

pub fn timestamp(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().buffer.timestamp),
    }
}

pub fn url(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().buffer.url),
    }
}

pub fn topic(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().buffer.topic),
    }
}

pub fn buffer_title_bar(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().buffer.topic),
    }
}

pub fn unread_indicator(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().general.unread_indicator),
    }
}

pub fn nickname(theme: &Theme, seed: Option<&str>, should_dim_nickname: bool) -> Style {
    let color = theme.colors().buffer.nickname;
    let calculate_alpha_color = |color: Color| -> Color {
        alpha_color(0.15, 0.61, theme.colors().buffer.background, color)
    };

    let Some(seed) = seed else {
        let color = if should_dim_nickname {
            calculate_alpha_color(color)
        } else {
            color
        };

        return Style { color: Some(color) };
    };

    let randomized_color = randomize_color(color, seed);
    let color = if should_dim_nickname {
        calculate_alpha_color(randomized_color)
    } else {
        randomized_color
    };

    Style { color: Some(color) }
}
