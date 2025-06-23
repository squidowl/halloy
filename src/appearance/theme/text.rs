use data::appearance::theme::{
    alpha_color, alpha_color_calculate, randomize_color,
};
use data::config::buffer::away;
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

pub fn url(theme: &Theme) -> Style {
    Style {
        color: Some(theme.colors().buffer.url),
    }
}

pub fn nickname<T: AsRef<str>>(
    theme: &Theme,
    seed: Option<T>,
    away_appearance: Option<away::Appearance>,
) -> Style {
    let nickname = theme.colors().buffer.nickname;
    let calculate_alpha_color = |color| {
        if let Some(away::Appearance::Dimmed(alpha)) = away_appearance {
            match alpha {
                // Calculate alpha based on background and foreground.
                None => alpha_color_calculate(
                    0.20,
                    0.61,
                    theme.colors().buffer.background,
                    color,
                ),
                // Calculate alpha based on user defined alpha value.
                Some(a) => alpha_color(color, a),
            }
        } else {
            color
        }
    };

    // If we have a seed we randomize the color based on the seed before adding any alpha value.
    let color = match seed {
        Some(seed) => {
            calculate_alpha_color(randomize_color(nickname, seed.as_ref()))
        }
        None => calculate_alpha_color(nickname),
    };

    Style { color: Some(color) }
}
