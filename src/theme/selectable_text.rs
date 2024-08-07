use data::{config, message, theme::hex_to_color, user::NickColor};

use crate::widget::selectable_text::{Catalog, Style, StyleFn};

use super::{text, Theme};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

pub fn default(theme: &Theme) -> Style {
    Style {
        color: None,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn transparent(theme: &Theme) -> Style {
    let color = text::transparent(theme).color;

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn info(theme: &Theme) -> Style {
    let color = text::info(theme).color;

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn accent(theme: &Theme) -> Style {
    let color = text::accent(theme).color;

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn server(
    theme: &Theme,
    server: Option<&message::source::Server>,
    config: &config::buffer::ServerMessages,
) -> Style {
    let color = server
        .and_then(|server| match server.kind() {
            message::source::server::Kind::Join => {
                config.join.hex.as_deref().and_then(hex_to_color)
            }
            message::source::server::Kind::Part => {
                config.part.hex.as_deref().and_then(hex_to_color)
            }
            message::source::server::Kind::Quit => {
                config.quit.hex.as_deref().and_then(hex_to_color)
            }
            message::source::server::Kind::ReplyTopic => {
                config.topic.hex.as_deref().and_then(hex_to_color)
            }
            message::source::server::Kind::ChangeHost => {
                config.change_host.hex.as_deref().and_then(hex_to_color)
            }
        })
        .or_else(|| text::info(theme).color);

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn nickname(theme: &Theme, nick_color: NickColor, transparent: bool) -> Style {
    let color = text::nickname(theme, nick_color, transparent).color;

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}

pub fn status(
    theme: &Theme,
    status: message::source::Status,
    config: &config::buffer::InternalMessages,
) -> Style {
    let color = match status {
        message::source::Status::Success => config
            .success
            .hex
            .as_deref()
            .and_then(hex_to_color)
            .or_else(|| text::success(theme).color),
        message::source::Status::Error => config
            .error
            .hex
            .as_deref()
            .and_then(hex_to_color)
            .or_else(|| text::error(theme).color),
    };

    Style {
        color,
        selection_color: theme.colors().accent.high_alpha,
    }
}
