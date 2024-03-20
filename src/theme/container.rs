use iced::widget::container::{Appearance, DefaultStyle, Status};
use iced::{Background, Border, Color};

use super::Theme;

impl DefaultStyle for Theme {
    fn default_style(&self, _status: Status) -> Appearance {
        Appearance::default()
    }
}

pub fn table_row(theme: &Theme, _status: Status, idx: usize) -> Appearance {
    let background = if idx % 2 != 0 {
        theme.colors().background.base
    } else {
        theme.colors().background.light
    };

    Appearance {
        background: Some(Background::Color(background)),
        text_color: Some(theme.colors().text.base),
        ..Default::default()
    }
}

pub fn primary(theme: &Theme, _status: Status) -> Appearance {
    Appearance {
        background: Some(Background::Color(theme.colors().background.base)),
        text_color: Some(theme.colors().text.base),
        ..Default::default()
    }
}

pub fn pane_body(theme: &Theme, _status: Status) -> Appearance {
    Appearance {
        background: Some(Background::Color(theme.colors().background.dark)),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

pub fn pane_body_selected(theme: &Theme, status: Status) -> Appearance {
    let pane_body = pane_body(theme, status);

    Appearance {
        border: Border {
            color: theme.colors().action.base,
            ..pane_body.border
        },
        ..pane_body
    }
}

pub fn pane_header(theme: &Theme, _status: Status) -> Appearance {
    Appearance {
        background: Some(Background::Color(theme.colors().background.darker)),
        border: Border {
            radius: [4.0, 4.0, 0.0, 0.0].into(),
            width: 1.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

pub fn command(_theme: &Theme, _status: Status) -> Appearance {
    Appearance {
        background: None,
        ..Default::default()
    }
}

pub fn command_selected(theme: &Theme, _status: Status) -> Appearance {
    Appearance {
        background: Some(Background::Color(theme.colors().background.darker)),
        border: Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn context(theme: &Theme, _status: Status) -> Appearance {
    Appearance {
        //TODO: Blur background when possible?
        background: Some(Background::Color(theme.colors().background.base)),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.colors().background.darker,
        },
        ..Default::default()
    }
}

pub fn highlight(theme: &Theme, _status: Status) -> Appearance {
    Appearance {
        background: Some(Background::Color(theme.colors().info.high_alpha)),
        border: Border {
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn semi_transparent(theme: &Theme, _status: Status) -> Appearance {
    Appearance {
        background: Some(
            Color {
                a: 0.80,
                ..theme.colors().background.base
            }
            .into(),
        ),
        ..Default::default()
    }
}
