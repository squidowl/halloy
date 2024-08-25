use data::{message, user::NickColor};

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
        selection_color: theme.colors().buffer.selection,
    }
}

pub fn action(theme: &Theme) -> Style {
    let color: Option<iced::Color> = text::action(theme).color;

    Style {
        color,
        selection_color: theme.colors().buffer.selection,
    }
}

pub fn tertiary(theme: &Theme) -> Style {
    let color = text::tertiary(theme).color;

    Style {
        color,
        selection_color: theme.colors().buffer.selection,
    }
}

pub fn timestamp(theme: &Theme) -> Style {
    let color = text::timestamp(theme).color;

    Style {
        color,
        selection_color: theme.colors().buffer.selection,
    }
}

pub fn topic(theme: &Theme) -> Style {
    let color = text::topic(theme).color;

    Style {
        color,
        selection_color: theme.colors().buffer.selection,
    }
}

pub fn server(theme: &Theme, server: Option<&message::source::Server>) -> Style {
    let colors = theme.colors().buffer.server_messages;
    let color = server
        .and_then(|server| match server.kind() {
            message::source::server::Kind::Join => colors.join,
            message::source::server::Kind::Part => colors.part,
            message::source::server::Kind::Quit => colors.quit,
            message::source::server::Kind::ReplyTopic => colors.reply_topic,
            message::source::server::Kind::ChangeHost => colors.change_host,
        })
        .or(Some(colors.default));

    Style {
        color,
        selection_color: theme.colors().buffer.selection,
    }
}

pub fn nickname(theme: &Theme, nick_color: NickColor, away: bool, away_transparency: f32) -> Style {
    let color = text::nickname(theme, nick_color, away, away_transparency).color;

    Style {
        color,
        selection_color: theme.colors().buffer.selection,
    }
}

pub fn status(theme: &Theme, status: message::source::Status) -> Style {
    let color = match status {
        message::source::Status::Success => text::success(theme).color,
        message::source::Status::Error => text::error(theme).color,
    };

    Style {
        color,
        selection_color: theme.colors().buffer.selection,
    }
}
