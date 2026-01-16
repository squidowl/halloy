use iced::widget::text_input::{Catalog, Status, Style, StyleFn};
use iced::{Background, Border, Color};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

pub fn primary(theme: &Theme, status: Status) -> Style {
    let active = Style {
        background: Background::Color(
            theme.styles().buttons.secondary.background,
        ),
        border: Border {
            radius: 4.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
            // XXX Not currently displayed in application.
        },
        icon: theme.styles().text.primary.color,
        placeholder: theme.styles().text.secondary.color,
        value: theme.styles().text.primary.color,
        selection: theme.styles().buffer.selection,
    };

    match status {
        Status::Active | Status::Focused { .. } => active,
        Status::Hovered => Style {
            background: Background::Color(
                theme.styles().buttons.secondary.background_hover,
            ),
            ..active
        },
        Status::Disabled => Style {
            background: Background::Color(Color {
                a: 0.2,
                ..theme.styles().buttons.secondary.background
            }),
            placeholder: Color {
                a: 0.2,
                ..theme.styles().text.secondary.color
            },
            ..active
        },
    }
}

pub fn error(theme: &Theme, status: Status) -> Style {
    let primary = primary(theme, status);

    match status {
        Status::Active | Status::Hovered | Status::Focused { .. } => Style {
            border: Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.styles().text.error.color,
            },
            ..primary
        },
        Status::Disabled => primary,
    }
}

// When the text_input does not need to indicate when it is disable (e.g. if the
// text_input is in a pane and is only disabled when the pane is being dragged)
// then this function can be used to hide the disabled state to avoid
// distracting the user
pub fn hide_disabled(
    style: impl Fn(&Theme, Status) -> Style,
    theme: &Theme,
    status: Status,
) -> Style {
    let status = if matches!(status, Status::Disabled) {
        Status::Active
    } else {
        status
    };

    style(theme, status)
}
