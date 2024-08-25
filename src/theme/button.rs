use iced::widget::button::{Catalog, Status, Style, StyleFn};
use iced::{Background, Border, Color};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

fn default(theme: &Theme, status: Status) -> Style {
    primary(theme, status, false)
}

fn button(
    button_colors: data::theme::Button,
    foreground: Color,
    status: Status,
    selected: bool,
) -> Style {
    match status {
        Status::Active | Status::Pressed => Style {
            background: Some(Background::Color(if selected {
                button_colors.background_selected
            } else {
                button_colors.background
            })),
            text_color: foreground,
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        Status::Hovered => Style {
            background: Some(Background::Color(if selected {
                button_colors.background_selected_hover
            } else {
                button_colors.background_hover
            })),
            text_color: foreground,
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        Status::Disabled => {
            let active: Style = button(button_colors, foreground, Status::Active, selected);

            Style {
                text_color: Color {
                    a: 0.2,
                    ..active.text_color
                },
                ..active
            }
        }
    }
}

pub fn primary(theme: &Theme, status: Status, selected: bool) -> Style {
    let button_colors = theme.colors().buttons.primary;
    let foreground_color = theme.colors().text.primary;

    button(button_colors, foreground_color, status, selected)
}

pub fn secondary(theme: &Theme, status: Status, selected: bool) -> Style {
    let button_colors = theme.colors().buttons.secondary;
    let foreground_color = theme.colors().text.primary;

    button(button_colors, foreground_color, status, selected)
}

pub fn bare(_theme: &Theme, status: Status) -> Style {
    match status {
        Status::Active | Status::Pressed | Status::Hovered => Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            ..Default::default()
        },
        Status::Disabled => {
            let active = bare(_theme, Status::Active);

            Style {
                text_color: Color {
                    a: 0.2,
                    ..active.text_color
                },
                border: Border {
                    color: Color {
                        a: 0.2,
                        ..active.text_color
                    },
                    ..Default::default()
                },
                ..active
            }
        }
    }
}
