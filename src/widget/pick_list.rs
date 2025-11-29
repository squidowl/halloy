use iced::widget::pick_list::{Catalog, Status, Style, StyleFn};
use iced::{Background, Border};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> StyleFn<'a, Self> {
        Box::new(default)
    }

    fn style(&self, class: &StyleFn<'_, Self>, status: Status) -> Style {
        class(self, status)
    }
}

fn default(theme: &Theme, status: Status) -> Style {
    Style {
        text_color: theme.styles().text.primary.color,
        placeholder_color: theme.styles().text.secondary.color,
        handle_color: theme.styles().text.primary.color,
        background: Background::Color(match status {
            Status::Active => theme.styles().buttons.secondary.background,
            Status::Hovered => {
                theme.styles().buttons.secondary.background_hover
            }
            Status::Opened { is_hovered } => {
                if is_hovered {
                    theme.styles().buttons.secondary.background_selected_hover
                } else {
                    theme.styles().buttons.secondary.background_selected
                }
            }
        }),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
    }
}
