use iced::{
    widget::{
        container,
        scrollable::{Catalog, Scrollbar, Scroller, Status, Style, StyleFn},
    },
    Background, Border, Color, Shadow,
};

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
        Status::Active | Status::Hovered { .. } | Status::Dragged { .. } => Style {
            container: container::Style {
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

pub fn hidden(_theme: &Theme, status: Status) -> Style {
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
        Status::Active | Status::Hovered { .. } | Status::Dragged { .. } => Style {
            container: container::Style {
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
