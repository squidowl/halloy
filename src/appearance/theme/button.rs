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
    foreground: Color,
    background: Color,
    background_hover: Color,
    background_pressed: Color,
    border_color: Option<Color>,
    status: Status,
) -> Style {
    let border = Border {
        radius: 4.0.into(),
        color: border_color.unwrap_or(Color::TRANSPARENT),
        width: 1.0,
    };

    match status {
        Status::Active => Style {
            background: Some(Background::Color(background)),
            text_color: foreground,
            border,
            ..Default::default()
        },
        Status::Hovered => Style {
            background: Some(Background::Color(background_hover)),
            text_color: foreground,
            border,
            ..Default::default()
        },
        Status::Pressed => Style {
            background: Some(Background::Color(background_pressed)),
            text_color: foreground,
            border,
            ..Default::default()
        },
        Status::Disabled => {
            let active: Style = button(
                foreground,
                background,
                background_hover,
                background_pressed,
                border_color,
                Status::Active,
            );

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

pub fn sidebar_buffer(
    theme: &Theme,
    status: Status,
    is_focused: bool,
    is_open: bool,
) -> Style {
    let foreground = theme.styles().text.primary.color;
    let button_colors = theme.styles().buttons.primary;

    let background = if is_open {
        button_colors.background_selected
    } else {
        button_colors.background
    };

    let background_hover = if is_open {
        button_colors.background_selected_hover
    } else {
        button_colors.background_hover
    };

    let background_pressed = button_colors.background_selected;

    button(
        foreground,
        background,
        background_hover,
        background_pressed,
        if matches!(status, Status::Pressed) {
            !is_focused
        } else {
            is_focused
        }
        .then_some(Color {
            a: 0.2,
            ..foreground
        }),
        status,
    )
}

pub fn primary(theme: &Theme, status: Status, selected: bool) -> Style {
    let foreground = theme.styles().text.primary.color;
    let button_colors = theme.styles().buttons.primary;

    let background = if selected {
        button_colors.background_selected
    } else {
        button_colors.background
    };

    let background_hover = if selected {
        button_colors.background_selected_hover
    } else {
        button_colors.background_hover
    };

    let background_pressed = if selected {
        button_colors.background_hover
    } else {
        button_colors.background_selected_hover
    };

    button(
        foreground,
        background,
        background_hover,
        background_pressed,
        None,
        status,
    )
}

pub fn secondary(theme: &Theme, status: Status, selected: bool) -> Style {
    let foreground = theme.styles().text.primary.color;
    let button_colors = theme.styles().buttons.secondary;

    let background = if selected {
        button_colors.background_selected
    } else {
        button_colors.background
    };

    let background_hover = if selected {
        button_colors.background_selected_hover
    } else {
        button_colors.background_hover
    };

    let background_pressed = if selected {
        button_colors.background_hover
    } else {
        button_colors.background_selected_hover
    };

    button(
        foreground,
        background,
        background_hover,
        background_pressed,
        None,
        status,
    )
}

pub fn picker(theme: &Theme, status: Status, is_selected: bool) -> Style {
    let foreground = theme.styles().text.primary.color;
    let button_colors = theme.styles().buttons.primary;

    let background = if is_selected {
        button_colors.background_hover
    } else {
        button_colors.background
    };

    let background_hover = if is_selected {
        button_colors.background_selected
    } else {
        button_colors.background_hover
    };

    let background_pressed = if is_selected {
        button_colors.background_selected_hover
    } else {
        button_colors.background_selected
    };

    button(
        foreground,
        background,
        background_hover,
        background_pressed,
        None,
        status,
    )
}

pub fn reaction(
    theme: &Theme,
    status: Status,
    already_reacted: bool,
    selection: bool,
) -> Style {
    let foreground = theme.styles().text.secondary.color;
    let button_colors = theme.styles().buttons.secondary;

    let background = if selection {
        button_colors.background_selected
    } else {
        button_colors.background
    };

    let background_hover = if selection {
        button_colors.background_selected_hover
    } else {
        button_colors.background_hover
    };

    let background_pressed = button_colors.background_selected;

    button(
        foreground,
        background,
        background_hover,
        background_pressed,
        if matches!(status, Status::Pressed) {
            !already_reacted
        } else {
            already_reacted
        }
        .then_some(foreground),
        status,
    )
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

pub fn reply_preview(theme: &Theme, status: Status) -> Style {
    let text = theme.styles().text;
    Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: match status {
            Status::Active | Status::Pressed => text.secondary.color,
            Status::Hovered => text.primary.color,
            Status::Disabled => Color {
                a: 0.2,
                ..text.secondary.color
            },
        },
        ..Default::default()
    }
}

pub fn preview_card(theme: &Theme, status: Status) -> Style {
    let foreground = theme.styles().text.primary.color;
    let background = theme.styles().buttons.secondary.background;
    let background_hover = theme.styles().buttons.secondary.background_hover;

    let border = Border {
        radius: 4.0.into(),
        width: 1.0,
        color: theme.styles().general.border,
    };

    match status {
        Status::Active | Status::Pressed => Style {
            background: Some(Background::Color(background)),
            text_color: foreground,
            border,
            ..Default::default()
        },
        Status::Hovered => Style {
            background: Some(Background::Color(background_hover)),
            text_color: foreground,
            border,
            ..Default::default()
        },
        // Not possible to disable this button style.
        Status::Disabled => self::primary(theme, status, false),
    }
}
