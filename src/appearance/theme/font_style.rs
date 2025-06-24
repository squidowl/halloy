use data::appearance::theme::FontStyle;
use data::message;
use data::message::source::server::{Kind, StandardReply};

use super::Theme;

pub fn default(_theme: &Theme) -> Option<FontStyle> {
    None
}

pub fn primary(theme: &Theme) -> Option<FontStyle> {
    theme.styles().text.primary.font_style
}

pub fn secondary(theme: &Theme) -> Option<FontStyle> {
    theme.styles().text.secondary.font_style
}

pub fn tertiary(theme: &Theme) -> Option<FontStyle> {
    theme.styles().text.tertiary.font_style
}

pub fn action(theme: &Theme) -> Option<FontStyle> {
    theme.styles().buffer.action.font_style
}

pub fn nickname(theme: &Theme) -> Option<FontStyle> {
    theme.styles().buffer.nickname.font_style
}

pub fn server(
    theme: &Theme,
    server: Option<&message::source::Server>,
) -> Option<FontStyle> {
    let styles = theme.styles().buffer.server_messages;
    server
        .and_then(|server| match server.kind() {
            Kind::Join => styles.join.font_style,
            Kind::Part => styles.part.font_style,
            Kind::Quit => styles.quit.font_style,
            Kind::ReplyTopic => styles.reply_topic.font_style,
            Kind::ChangeHost => styles.change_host.font_style,
            Kind::MonitoredOnline => styles.monitored_online.font_style,
            Kind::MonitoredOffline => styles.monitored_offline.font_style,
            Kind::StandardReply(StandardReply::Fail) => styles
                .standard_reply_fail
                .font_style
                .or(theme.styles().text.error.font_style),
            Kind::StandardReply(StandardReply::Warn) => styles
                .standard_reply_warn
                .font_style
                .or(theme.styles().text.error.font_style),
            Kind::StandardReply(StandardReply::Note) => {
                styles.standard_reply_note.font_style
            }
            Kind::Wallops => styles.wallops.font_style,
        })
        .or(styles.default.font_style)
}

pub fn status(
    theme: &Theme,
    status: message::source::Status,
) -> Option<FontStyle> {
    match status {
        message::source::Status::Success => success(theme),
        message::source::Status::Error => error(theme),
    }
}

pub fn error(theme: &Theme) -> Option<FontStyle> {
    theme.styles().text.error.font_style
}

pub fn success(theme: &Theme) -> Option<FontStyle> {
    theme.styles().text.success.font_style
}

pub fn timestamp(theme: &Theme) -> Option<FontStyle> {
    theme.styles().buffer.timestamp.font_style
}

pub fn topic(theme: &Theme) -> Option<FontStyle> {
    theme.styles().buffer.topic.font_style
}

pub fn buffer_title_bar(theme: &Theme) -> Option<FontStyle> {
    theme.styles().buffer.topic.font_style
}

pub fn url(theme: &Theme) -> Option<FontStyle> {
    theme.styles().buffer.url.font_style
}
