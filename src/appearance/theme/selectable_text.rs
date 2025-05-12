use data::config::buffer::away;
use data::message::source::server::{Kind, StandardReply};
use data::{Config, User, log, message};

use super::{Theme, text};
use crate::widget::selectable_rich_text;
use crate::widget::selectable_text::{Catalog, Style, StyleFn};

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
        selection_color: theme.styles().buffer.selection,
    }
}

pub fn action(theme: &Theme) -> Style {
    let color: Option<iced::Color> = text::action(theme).color;

    Style {
        color,
        selection_color: theme.styles().buffer.selection,
    }
}

pub fn tertiary(theme: &Theme) -> Style {
    let color = text::tertiary(theme).color;

    Style {
        color,
        selection_color: theme.styles().buffer.selection,
    }
}

pub fn timestamp(theme: &Theme) -> Style {
    let color = text::timestamp(theme).color;

    Style {
        color,
        selection_color: theme.styles().buffer.selection,
    }
}

pub fn topic(theme: &Theme) -> Style {
    let color = text::topic(theme).color;

    Style {
        color,
        selection_color: theme.styles().buffer.selection,
    }
}

pub fn server(
    theme: &Theme,
    server: Option<&message::source::Server>,
) -> Style {
    let styles = theme.styles().buffer.server_messages;
    let color = server
        .and_then(|server| match server.kind() {
            Kind::Join => styles.join,
            Kind::Part => styles.part,
            Kind::Quit => styles.quit,
            Kind::ReplyTopic => styles.reply_topic,
            Kind::ChangeHost => styles.change_host,
            Kind::ChangeMode => styles.change_mode,
            Kind::ChangeNick => styles.change_nick,
            Kind::MonitoredOnline => styles.monitored_online,
            Kind::MonitoredOffline => styles.monitored_offline,
            Kind::StandardReply(StandardReply::Fail) => styles
                .standard_reply_fail
                .or(Some(theme.styles().text.error)),
            Kind::StandardReply(StandardReply::Warn) => styles
                .standard_reply_warn
                .or(theme.styles().text.warning)
                .or(Some(theme.styles().text.error)),
            Kind::StandardReply(StandardReply::Note) => {
                colors.standard_reply_note.or(theme.styles().text.info)
            }
            Kind::Wallops => styles.wallops,
        })
        .or(Some(styles.default))
        .map(|style| style.color);

    Style {
        color,
        selection_color: theme.styles().buffer.selection,
    }
}

pub fn nicklist_nickname(theme: &Theme, config: &Config, user: &User) -> Style {
    nickname_style(
        theme,
        config.buffer.channel.nicklist.color,
        user,
        config.buffer.away.appearance(user.is_away()),
    )
}

pub fn nickname(theme: &Theme, config: &Config, user: &User) -> Style {
    nickname_style(
        theme,
        config.buffer.channel.message.nickname_color,
        user,
        config.buffer.away.appearance(user.is_away()),
    )
}

pub fn topic_nickname(theme: &Theme, config: &Config, user: &User) -> Style {
    nickname_style(
        theme,
        config.buffer.channel.message.nickname_color,
        user,
        None,
    )
}

fn nickname_style(
    theme: &Theme,
    kind: data::buffer::Color,
    user: &User,
    away_appearance: Option<away::Appearance>,
) -> Style {
    let seed = match kind {
        data::buffer::Color::Solid => None,
        data::buffer::Color::Unique => Some(user.seed()),
    };

    let color = text::nickname(theme, seed, away_appearance).color;

    Style {
        color,
        selection_color: theme.styles().buffer.selection,
    }
}

pub fn status(theme: &Theme, status: message::source::Status) -> Style {
    let color = match status {
        message::source::Status::Success => text::success(theme).color,
        message::source::Status::Error => text::error(theme).color,
    };

    Style {
        color,
        selection_color: theme.styles().buffer.selection,
    }
}

pub fn log_level(theme: &Theme, log_level: log::Level) -> Style {
    let color = match log_level {
        log::Level::Error => theme.colors().text.error,
        log::Level::Warn => theme
            .colors()
            .text
            .warning
            .unwrap_or(theme.colors().general.unread_indicator),
        log::Level::Info => theme
            .colors()
            .text
            .info
            .unwrap_or(theme.colors().buffer.server_messages.default),
        log::Level::Debug => theme
            .colors()
            .text
            .debug
            .unwrap_or(theme.colors().buffer.code),
        log::Level::Trace => theme
            .colors()
            .text
            .trace
            .unwrap_or(theme.colors().text.secondary),
    };

    Style {
        color: Some(color),
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
