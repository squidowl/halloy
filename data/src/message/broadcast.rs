//! Generate messages that can be broadcast into every buffer
use chrono::{DateTime, Utc};

use super::{parse_fragments, plain, source, Content, Direction, Message, Source, Target};
use crate::config::buffer::UsernameFormat;
use crate::time::Posix;
use crate::user::Nick;
use crate::{Config, User};

enum Cause {
    Server(Option<source::Server>),
    Status(source::Status),
}

fn expand(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
    include_server: bool,
    cause: Cause,
    content: Content,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let message = |target, content| -> Message {
        Message {
            received_at: Posix::now(),
            server_time: sent_time,
            direction: Direction::Received,
            target,
            content,
        }
    };

    let source = match cause {
        Cause::Server(server) => Source::Server(server),
        Cause::Status(status) => Source::Internal(source::Internal::Status(status)),
    };

    channels
        .into_iter()
        .map(|channel| {
            message(
                Target::Channel {
                    channel,
                    source: source.clone(),
                },
                content.clone(),
            )
        })
        .chain(queries.into_iter().map(|nick| {
            message(
                Target::Query {
                    nick,
                    source: source.clone(),
                },
                content.clone(),
            )
        }))
        .chain(include_server.then(|| {
            message(
                Target::Server {
                    source: source.clone(),
                },
                content.clone(),
            )
        }))
        .collect()
}

pub fn connecting(sent_time: DateTime<Utc>) -> Vec<Message> {
    let content = plain(" ∙ connecting to server...".into());
    expand(
        [],
        [],
        true,
        Cause::Status(source::Status::Success),
        content,
        sent_time,
    )
}

pub fn connected(sent_time: DateTime<Utc>) -> Vec<Message> {
    let content = plain(" ∙ connected".into());
    expand(
        [],
        [],
        true,
        Cause::Status(source::Status::Success),
        content,
        sent_time,
    )
}

pub fn connection_failed(error: String, sent_time: DateTime<Utc>) -> Vec<Message> {
    let content = plain(format!(" ∙ connection to server failed ({error})"));
    expand(
        [],
        [],
        true,
        Cause::Status(source::Status::Error),
        content,
        sent_time,
    )
}

pub fn disconnected(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
    error: Option<String>,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let error = error.map(|error| format!(" ({error})")).unwrap_or_default();
    let content = plain(format!(" ∙ connection to server lost{error}"));
    expand(
        channels,
        queries,
        true,
        Cause::Status(source::Status::Error),
        content,
        sent_time,
    )
}

pub fn reconnected(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let content = plain(" ∙ connection to server restored".into());
    expand(
        channels,
        queries,
        true,
        Cause::Status(source::Status::Success),
        content,
        sent_time,
    )
}

pub fn quit(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
    user: &User,
    comment: &Option<String>,
    config: &Config,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let comment = comment
        .as_ref()
        .map(|comment| format!(" ({comment})"))
        .unwrap_or_default();

    let content = parse_fragments(format!(
        "⟵ {} has quit{comment}",
        user.formatted(config.buffer.server_messages.quit.username_format)
    ));

    expand(
        channels,
        queries,
        false,
        Cause::Server(Some(source::Server::new(
            source::server::Kind::Quit,
            Some(user.nickname().to_owned()),
        ))),
        content,
        sent_time,
    )
}

pub fn nickname(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
    old_nick: &Nick,
    new_nick: &Nick,
    ourself: bool,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let content = if ourself {
        plain(format!(" ∙ You're now known as {new_nick}"))
    } else {
        plain(format!(" ∙ {old_nick} is now known as {new_nick}"))
    };

    expand(
        channels,
        queries,
        false,
        Cause::Server(None),
        content,
        sent_time,
    )
}

pub fn invite(
    inviter: Nick,
    channel: String,
    channels: impl IntoIterator<Item = String>,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let content = plain(format!(" ∙ {inviter} invited you to join {channel}"));

    expand(channels, [], false, Cause::Server(None), content, sent_time)
}

pub fn change_host(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
    old_user: &User,
    new_username: &str,
    new_hostname: &str,
    ourself: bool,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let content = if ourself {
        plain(format!(
            " ∙ You've changed host to {new_username}@{new_hostname}",
        ))
    } else {
        plain(format!(
            " ∙ {} changed host to {new_username}@{new_hostname}",
            old_user.formatted(UsernameFormat::Full)
        ))
    };

    expand(
        channels,
        queries,
        false,
        Cause::Server(None),
        content,
        sent_time,
    )
}
