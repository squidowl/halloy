use data::config::buffer::Dimmed;
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

pub fn self_message(theme: &Theme) -> Style {
    Style {
        color: Some(
            theme
                .styles()
                .buffer
                .self_message
                .color
                .unwrap_or(theme.styles().text.primary.color),
        ),
        selection_color: theme.styles().buffer.selection,
    }
}

pub fn secondary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().text.secondary.color),
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
            Kind::JoinTopic => styles.join_topic.color,
            Kind::RequestTopic => styles.request_topic.color,
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
            Kind::Away => styles.away.color,
            Kind::Invite => styles.invite.color,
        })
        .or(Some(styles.default.color));

    Style {
        color,
        selection_color: theme.styles().buffer.selection,
    }
}

pub fn nickname(
    theme: &Theme,
    config: &Config,
    user: &User,
    metadata_color: Option<Color>,
    is_away: bool,
    is_user_offline: bool,
) -> Style {
    let offline_style = config.buffer.nickname.offline.style(is_user_offline);
    let away_alpha = config.buffer.nickname.away.alpha(is_away);
    let color_override = config
        .buffer
        .nickname
        .color_override(user.nickname().as_str())
        .or(metadata_color);
    let color = text::nickname(
        theme,
        &config.buffer.nickname.color,
        Some(user.seed()),
        color_override,
        away_alpha,
        offline_style,
    )
    .color;

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
            data::message::Link::User(..)
            | data::message::Link::Channel(..)
            | data::message::Link::GoToMessage(..)
            | data::message::Link::ExpandMessage(..)
            | data::message::Link::ContractMessage(..) => false,
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

pub fn color_dot(theme: &Theme, color: Color) -> Style {
    Style {
        color: Some(color),
        selection_color: theme.styles().buffer.selection,
    }
}
