use data::appearance::theme::{
    alpha_color, alpha_color_calculate, randomize_color,
};
use data::config::buffer;
use iced::widget::text::{Catalog, Style, StyleFn};

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
        color: Some(theme.styles().text.primary.color),
    }
}

pub fn secondary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().text.secondary.color),
    }
}

pub fn tertiary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().text.tertiary.color),
    }
}

pub fn error(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().text.error.color),
    }
}

pub fn success(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().text.success.color),
    }
}

pub fn action(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().buffer.action.color),
    }
}

pub fn timestamp(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().buffer.timestamp.color),
    }
}

pub fn topic(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().buffer.topic.color),
    }
}

pub fn buffer_title_bar(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().buffer.topic.color),
    }
}

pub fn unread_indicator(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().general.unread_indicator),
    }
}

pub fn url(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().buffer.url.color),
    }
}

pub fn nickname<T: AsRef<str>>(
    theme: &Theme,
    seed: Option<T>,
    is_away: Option<buffer::Away>,
    is_offline: bool,
) -> Style {
    let calculate_alpha_color = |color| {
        if let Some(buffer::Away::Dimmed(alpha)) = is_away {
            match alpha {
                // Calculate alpha based on background and foreground.
                None => alpha_color_calculate(
                    0.20,
                    0.61,
                    theme.styles().buffer.background,
                    color,
                ),
                // Calculate alpha based on user defined alpha value.
                Some(a) => alpha_color(color, a),
            }
        } else {
            color
        }
    };

    // If the user is offline, use the offline style if it exists
    if is_offline && let Some(style) = theme.styles().buffer.nickname_offline {
        return Style {
            color: Some(calculate_alpha_color(style.color)),
        };
    }

    let nickname = theme.styles().buffer.nickname;

    // If we have a seed we randomize the color based on the seed before adding any alpha value.
    let color = match seed {
        Some(seed) => calculate_alpha_color(randomize_color(
            nickname.color,
            seed.as_ref(),
        )),
        None => calculate_alpha_color(nickname.color),
    };

    Style { color: Some(color) }
}
