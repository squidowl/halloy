//! Generate messages that can be broadcast into every buffer
use std::collections::HashSet;

use chrono::{DateTime, Utc};

use super::{
    Content, Direction, Message, Source, Target, parse_fragments_with_user, plain, source,
};
use crate::config::buffer::UsernameFormat;
use crate::time::Posix;
use crate::user::Nick;
use crate::{Config, User, message, target};

enum Cause {
    Server(Option<source::Server>),
    Status(source::Status),
}

fn expand(
    channels: impl IntoIterator<Item = target::Channel>,
    queries: impl IntoIterator<Item = target::Query>,
    include_server: bool,
    cause: Cause,
    content: Content,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let message = |target, content| -> Message {
        let hash = message::Hash::new(&sent_time, &content);

        Message {
            received_at: Posix::now(),
            server_time: sent_time,
            direction: Direction::Received,
            target,
            content,
            id: None,
            hash,
            hidden_urls: HashSet::default(),
            is_echo: false,
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
                    channel: channel.clone(),
                    source: source.clone(),
                },
                content.clone(),
            )
        })
        .chain(queries.into_iter().map(|query| {
            message(
                Target::Query {
                    query: query.clone(),
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
    let content = plain("connecting to server...".into());
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
    let content = plain("connected".into());
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
    let content = plain(format!("connection to server failed ({error})"));
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
    channels: impl IntoIterator<Item = target::Channel>,
    queries: impl IntoIterator<Item = target::Query>,
    error: Option<String>,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let error = error.map(|error| format!(" ({error})")).unwrap_or_default();
    let content = plain(format!("connection to server lost{error}"));
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
    channels: impl IntoIterator<Item = target::Channel>,
    queries: impl IntoIterator<Item = target::Query>,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let content = plain("connection to server restored".into());
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
    channels: impl IntoIterator<Item = target::Channel>,
    queries: impl IntoIterator<Item = target::Query>,
    user: &User,
    comment: &Option<String>,
    config: &Config,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let comment = comment
        .as_ref()
        .map(|comment| format!(" ({comment})"))
        .unwrap_or_default();

    let content = parse_fragments_with_user(
        format!(
            "⟵ {} has quit{comment}",
            user.formatted(config.buffer.server_messages.quit.username_format)
        ),
        user,
    );

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
    channels: impl IntoIterator<Item = target::Channel>,
    queries: impl IntoIterator<Item = target::Query>,
    old_nick: &Nick,
    new_nick: &Nick,
    ourself: bool,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let content = if ourself {
        plain(format!("You're now known as {new_nick}"))
    } else {
        plain(format!("{old_nick} is now known as {new_nick}"))
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
    channel: target::Channel,
    channels: impl IntoIterator<Item = target::Channel>,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let inviter = User::from(inviter);
    let content = parse_fragments_with_user(
        format!("{} invited you to join {channel}", inviter.nickname()),
        &inviter,
    );

    expand(channels, [], false, Cause::Server(None), content, sent_time)
}

pub fn change_host(
    channels: impl IntoIterator<Item = target::Channel>,
    queries: impl IntoIterator<Item = target::Query>,
    old_user: &User,
    new_username: &str,
    new_hostname: &str,
    ourself: bool,
    logged_in: bool,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let content = if ourself {
        plain(format!(
            "You've changed host to {new_username}@{new_hostname}",
        ))
    } else {
        parse_fragments_with_user(
            format!(
                "{} changed host to {new_username}@{new_hostname}",
                old_user.formatted(UsernameFormat::Full)
            ),
            old_user,
        )
    };

    if ourself && !logged_in {
        expand(
            [],
            [],
            true,
            Cause::Server(Some(source::Server::new(
                source::server::Kind::ChangeHost,
                Some(old_user.nickname().to_owned()),
            ))),
            content,
            sent_time,
        )
    } else {
        expand(
            channels,
            queries,
            false,
            Cause::Server(Some(source::Server::new(
                source::server::Kind::ChangeHost,
                Some(old_user.nickname().to_owned()),
            ))),
            content,
            sent_time,
        )
    }
}
