use iced::{
    widget::{
        container,
        scrollable::{Appearance, DefaultStyle, Scrollbar, Scroller, Status},
    },
    Background, Border, Color, Shadow,
};

use super::Theme;

impl DefaultStyle for Theme {
    fn default_style(&self, status: Status) -> Appearance {
        primary(self, status)
    }
}

pub fn primary(theme: &Theme, status: Status) -> Appearance {
    let scrollbar = Scrollbar {
        background: None,
        border: Border::default(),
        scroller: Scroller {
            color: theme.colors().background.darker,
            border: Border {
                radius: 8.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        },
    };

    match status {
        Status::Active | Status::Hovered { .. } | Status::Dragged { .. } => Appearance {
            container: container::Appearance {
                text_color: None,
                background: None,
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: Color::TRANSPARENT,
                },
                shadow: Shadow::default(),
            },
            vertical_scrollbar: scrollbar,
            horizontal_scrollbar: scrollbar,
            gap: None,
        },
    }
}

pub fn hidden(_theme: &Theme, status: Status) -> Appearance {
    let scrollbar = Scrollbar {
        background: None,
        border: Border::default(),
        scroller: Scroller {
            color: Color::TRANSPARENT,
            border: Border {
                radius: 0.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        },
    };

    match status {
        Status::Active | Status::Hovered { .. } | Status::Dragged { .. } => Appearance {
            container: container::Appearance {
                text_color: None,
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: Color::TRANSPARENT,
                },
                shadow: Shadow::default(),
            },
            vertical_scrollbar: scrollbar,
            horizontal_scrollbar: scrollbar,
            gap: None,
        },
    }
}
