use data::config::buffer::{self, Dimmed};
use data::message::source::server::{Kind, StandardReply};
use data::{Config, User, log, message};
use iced::Color;
use iced::theme::Base;

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

pub fn logs(theme: &Theme) -> Style {
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

pub fn condensed_marker(theme: &Theme) -> Style {
    let color = text::timestamp(theme).color;

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
            Kind::Join => styles.join.color,
            Kind::Part => styles.part.color,
            Kind::Quit => styles.quit.color,
            Kind::ReplyTopic => styles.reply_topic.color,
            Kind::ChangeHost => styles.change_host.color,
            Kind::ChangeMode => styles.change_mode.color,
            Kind::ChangeNick => styles.change_nick.color,
            Kind::ChangeTopic => styles.change_topic.color,
            Kind::MonitoredOnline => styles.monitored_online.color,
            Kind::MonitoredOffline => styles.monitored_offline.color,
            Kind::StandardReply(StandardReply::Fail) => styles
                .standard_reply_fail
                .color
                .or(Some(theme.styles().text.error.color)),
            Kind::StandardReply(StandardReply::Warn) => styles
                .standard_reply_warn
                .color
                .or(theme.styles().text.warning.color)
                .or(Some(theme.styles().text.error.color)),
            Kind::StandardReply(StandardReply::Note) => {
                styles.standard_reply_note.color
            }
            Kind::WAllOps => styles.wallops.color,
            Kind::Kick => styles.kick.color,
        })
        .or(Some(styles.default.color));

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
        config.buffer.channel.nicklist.away.is_away(user.is_away()),
        false,
    )
}

pub fn nickname(
    theme: &Theme,
    config: &Config,
    user: &User,
    is_user_offline: bool,
) -> Style {
    nickname_style(
        theme,
        config.buffer.channel.message.nickname_color,
        user,
        config
            .buffer
            .nickname
            .away
            .is_away(user.is_away() || is_user_offline),
        config.buffer.nickname.offline.is_offline(is_user_offline),
    )
}

pub fn topic_nickname(theme: &Theme, config: &Config, user: &User) -> Style {
    nickname_style(
        theme,
        config.buffer.channel.message.nickname_color,
        user,
        None,
        false,
    )
}

fn nickname_style(
    theme: &Theme,
    kind: data::buffer::Color,
    user: &User,
    is_away: Option<buffer::Away>,
    is_offline: bool,
) -> Style {
    let seed = match kind {
        data::buffer::Color::Solid => None,
        data::buffer::Color::Unique => Some(user.seed()),
    };

    let color = text::nickname(theme, seed, is_away, is_offline).color;

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
        log::Level::Error => theme.styles().text.error.color,
        log::Level::Warn => theme
            .styles()
            .text
            .warning
            .color
            .unwrap_or(theme.styles().general.unread_indicator),
        log::Level::Info => theme
            .styles()
            .text
            .info
            .color
            .unwrap_or(theme.styles().buffer.server_messages.default.color),
        log::Level::Debug => theme
            .styles()
            .text
            .debug
            .color
            .unwrap_or(theme.styles().buffer.code.color),
        log::Level::Trace => theme
            .styles()
            .text
            .trace
            .color
            .unwrap_or(theme.styles().text.secondary.color),
    };

    Style {
        color: Some(color),
        selection_color: theme.styles().buffer.selection,
    }
}

impl selectable_rich_text::Link for message::Link {
    fn underline(&self) -> bool {
        match self {
            data::message::Link::Url(_) => true,
            data::message::Link::User(_)
            | data::message::Link::Channel(_)
            | data::message::Link::GoToMessage(..)
            | data::message::Link::ExpandCondensedMessage(..)
            | data::message::Link::ContractCondensedMessage(..) => false,
        }
    }
}

pub fn dimmed(
    style: Style,
    theme: &Theme,
    dimmed: Option<(Dimmed, Color)>,
) -> Style {
    if let Some((dimmed, background)) = dimmed {
        Style {
            color: Some(dimmed.transform_color(
                style.color.unwrap_or(theme.base().text_color),
                background,
            )),
            selection_color: style.selection_color,
        }
    } else {
        style
    }
}
