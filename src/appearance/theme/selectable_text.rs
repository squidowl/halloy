use data::{message, Config, User};

use crate::widget::{
    selectable_rich_text,
    selectable_text::{Catalog, Style, StyleFn},
};

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
            message::source::server::Kind::MonitoredOnline => colors.monitored_online,
            message::source::server::Kind::MonitoredOffline => colors.monitored_offline,
        })
        .or(Some(colors.default));

    Style {
        color,
        selection_color: theme.colors().buffer.selection,
    }
}

pub fn nicklist_nickname(theme: &Theme, config: &Config, user: &User) -> Style {
    nickname_style(
        theme,
        config.buffer.channel.nicklist.color,
        user,
        config.buffer.away.should_dim_nickname(user.is_away()),
    )
}

pub fn nickname(theme: &Theme, config: &Config, user: &User) -> Style {
    nickname_style(
        theme,
        config.buffer.channel.message.nickname_color,
        user,
        config.buffer.away.should_dim_nickname(user.is_away()),
    )
}

pub fn topic_nickname(theme: &Theme, config: &Config, user: &User) -> Style {
    nickname_style(
        theme,
        config.buffer.channel.message.nickname_color,
        user,
        false,
    )
}

fn nickname_style(
    theme: &Theme,
    kind: data::buffer::Color,
    user: &User,
    should_dim_nickname: bool,
) -> Style {
    let seed = match kind {
        data::buffer::Color::Solid => None,
        data::buffer::Color::Unique => Some(user.seed()),
    };

    let color = text::nickname(theme, seed, should_dim_nickname).color;

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

impl selectable_rich_text::Link for message::Link {
    fn underline(&self) -> bool {
        match self {
            data::message::Link::Url(_) => true,
            data::message::Link::User(_)
            | data::message::Link::Channel(_)
            | data::message::Link::GoToMessage(..) => false,
        }
    }
}
