use data::appearance::theme::{nickname_alpha, nickname_color};
use data::config::buffer;
use data::message;
use data::message::source::server::{Kind, StandardReply};
use iced::widget::text::{Catalog, Style, StyleFn};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(none)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

pub fn none(_theme: &Theme) -> Style {
    Style { color: None }
}

pub fn primary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().text.primary.color),
    }
}

pub fn secondary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().text.secondary.color),
    }
}

pub fn tertiary(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().text.tertiary.color),
    }
}

pub fn error(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().text.error.color),
    }
}

pub fn warning(theme: &Theme) -> Style {
    Style {
        color: theme
            .styles()
            .text
            .warning
            .color
            .or(Some(theme.styles().text.error.color)),
    }
}

pub fn success(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().text.success.color),
    }
}

pub fn action(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().buffer.action.color),
    }
}

pub fn timestamp(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().buffer.timestamp.color),
    }
}

pub fn topic(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().buffer.topic.color),
    }
}

pub fn buffer_title_bar(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().buffer.topic.color),
    }
}

pub fn unread_indicator(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().general.unread_indicator),
    }
}

pub fn highlight_indicator(theme: &Theme) -> Style {
    Style {
        color: theme
            .styles()
            .general
            .highlight_indicator
            .or(Some(theme.styles().general.unread_indicator)),
    }
}

pub fn backlog(theme: &Theme) -> Style {
    Style {
        color: Some(
            theme
                .styles()
                .buffer
                .backlog_rule
                .unwrap_or(theme.styles().general.horizontal_rule),
        ),
    }
}

pub fn url(theme: &Theme) -> Style {
    Style {
        color: Some(theme.styles().buffer.url.color),
    }
}

pub fn nickname(
    theme: &Theme,
    kind: &data::buffer::Color,
    seed: Option<&str>,
    is_away: Option<buffer::Away>,
    is_offline: bool,
) -> Style {
    let color = nickname_alpha(
        if is_offline
            && let Some(offline_color) =
                theme.styles().buffer.nickname_offline.color
        {
            offline_color
        } else {
            let nickname = theme.styles().buffer.nickname;

            nickname_color(nickname.color, kind, seed)
        },
        is_away,
        theme.styles().buffer.background,
    );

    Style { color: Some(color) }
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
            Kind::Away => styles.away.color,
            Kind::Invite => styles.invite.color,
        })
        .or(Some(styles.default.color));

    Style { color }
}
