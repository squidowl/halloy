use data::appearance::theme::FontStyle;
use data::message;
use data::message::source::server::{Kind, StandardReply};

use super::Theme;

pub fn default(_theme: &Theme) -> FontStyle {
    FontStyle::default()
}

pub fn tertiary(theme: &Theme) -> FontStyle {
    theme.styles().text.tertiary.font_style
}

pub fn action(theme: &Theme) -> FontStyle {
    theme.styles().buffer.action.font_style
}

pub fn nickname(theme: &Theme) -> FontStyle {
    theme.styles().buffer.nickname.font_style
}

pub fn server(
    theme: &Theme,
    server: Option<&message::source::Server>,
) -> FontStyle {
    let styles = theme.styles().buffer.server_messages;
    server
        .and_then(|server| match server.kind() {
            Kind::Join => styles.join,
            Kind::Part => styles.part,
            Kind::Quit => styles.quit,
            Kind::ReplyTopic => styles.reply_topic,
            Kind::ChangeHost => styles.change_host,
            Kind::MonitoredOnline => styles.monitored_online,
            Kind::MonitoredOffline => styles.monitored_offline,
            Kind::StandardReply(StandardReply::Fail) => styles
                .standard_reply_fail
                .or(Some(theme.styles().text.error)),
            Kind::StandardReply(StandardReply::Warn) => styles
                .standard_reply_warn
                .or(Some(theme.styles().text.error)),
            Kind::StandardReply(StandardReply::Note) => {
                styles.standard_reply_note
            }
            Kind::Wallops => styles.wallops,
        })
        .map_or(styles.default.font_style, |style| style.font_style)
}

pub fn status(theme: &Theme, status: message::source::Status) -> FontStyle {
    match status {
        message::source::Status::Success => success(theme),
        message::source::Status::Error => error(theme),
    }
}

pub fn error(theme: &Theme) -> FontStyle {
    theme.styles().text.error.font_style
}

pub fn success(theme: &Theme) -> FontStyle {
    theme.styles().text.success.font_style
}

pub fn timestamp(theme: &Theme) -> FontStyle {
    theme.styles().buffer.timestamp.font_style
}

pub fn topic(theme: &Theme) -> FontStyle {
    theme.styles().buffer.topic.font_style
}
