//! Generate messages that can be broadcast into every buffer
use std::collections::HashSet;

use chrono::{DateTime, Utc};

use super::{
    Content, Direction, Message, Source, Target, kick_text,
    parse_fragments_with_user, parse_fragments_with_users, plain, source,
};
use crate::config::buffer::UsernameFormat;
use crate::time::Posix;
use crate::user::Nick;
use crate::{Config, User, isupport, message, target};

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
            blocked: false,
            condensed: None,
            expanded: false,
            command: None,
        }
    };

    let source = match cause {
        Cause::Server(server) => Source::Server(server),
        Cause::Status(status) => {
            Source::Internal(source::Internal::Status(status))
        }
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

pub fn connection_failed(
    error: String,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
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
    casemapping: isupport::CaseMap,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let comment = comment
        .as_ref()
        .map(|comment| format!(" ({comment})"))
        .unwrap_or_default();

    let content = parse_fragments_with_user(
        format!(
            "⟵ {} has quit{comment}",
            user.formatted(
                config
                    .buffer
                    .server_messages
                    .username_format(Some(source::server::Kind::Quit))
            )
        ),
        user,
        casemapping,
    );

    expand(
        channels,
        queries,
        false,
        Cause::Server(Some(source::Server::new(
            source::server::Kind::Quit,
            Some(user.nickname().to_owned()),
            None,
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
    casemapping: isupport::CaseMap,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let old_user = User::from(old_nick.clone());
    let new_user = User::from(new_nick.clone());

    let cause = Cause::Server(Some(source::Server::new(
        source::server::Kind::ChangeNick,
        Some(old_user.nickname().to_owned()),
        Some(source::server::Change::Nick(new_user.nickname().to_owned())),
    )));

    let content = if ourself {
        parse_fragments_with_user(
            format!("You're now known as {new_nick}"),
            &new_user,
            casemapping,
        )
    } else {
        parse_fragments_with_users(
            format!("{old_nick} is now known as {new_nick}"),
            Some(&[old_user.clone(), new_user].into_iter().collect()),
            casemapping,
        )
    };

    expand(channels, queries, false, cause, content, sent_time)
}

pub fn invite(
    inviter: Nick,
    channel: target::Channel,
    channels: impl IntoIterator<Item = target::Channel>,
    casemapping: isupport::CaseMap,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let inviter = User::from(inviter);
    let content = parse_fragments_with_user(
        format!("{} invited you to join {channel}", inviter.nickname()),
        &inviter,
        casemapping,
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
    casemapping: isupport::CaseMap,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let cause = Cause::Server(Some(source::Server::new(
        source::server::Kind::ChangeHost,
        Some(old_user.nickname().to_owned()),
        old_user.hostname().map(|old_hostname| {
            source::server::Change::Host(
                old_hostname.to_string(),
                new_hostname.to_string(),
            )
        }),
    )));

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
            casemapping,
        )
    };

    if ourself && !logged_in {
        expand([], [], true, cause, content, sent_time)
    } else {
        expand(channels, queries, false, cause, content, sent_time)
    }
}

pub fn kick(
    kicker: User,
    victim: User,
    reason: Option<String>,
    channel: target::Channel,
    casemapping: isupport::CaseMap,
    sent_time: DateTime<Utc>,
) -> Vec<Message> {
    let cause = Cause::Server(Some(source::Server::new(
        source::server::Kind::Kick,
        Some(kicker.nickname().to_owned()),
        None,
    )));

    let content = kick_text(
        kicker,
        victim,
        true, // Broadcast of KICK is always ourself
        &reason,
        Some(channel),
        casemapping,
    );

    expand([], [], true, cause, content, sent_time)
}

#[derive(Debug, Clone)]
pub enum Broadcast {
    Connecting,
    Connected,
    ConnectionFailed {
        error: String,
    },
    Disconnected {
        error: Option<String>,
    },
    Reconnected,
    Quit {
        user: User,
        comment: Option<String>,
        user_channels: Vec<target::Channel>,
        casemapping: isupport::CaseMap,
    },
    Nickname {
        old_nick: Nick,
        new_nick: Nick,
        ourself: bool,
        user_channels: Vec<target::Channel>,
        casemapping: isupport::CaseMap,
    },
    Invite {
        inviter: Nick,
        channel: target::Channel,
        user_channels: Vec<target::Channel>,
        casemapping: isupport::CaseMap,
    },
    ChangeHost {
        old_user: User,
        new_username: String,
        new_hostname: String,
        ourself: bool,
        logged_in: bool,
        user_channels: Vec<target::Channel>,
        casemapping: isupport::CaseMap,
    },
    Kick {
        kicker: User,
        victim: User,
        reason: Option<String>,
        channel: target::Channel,
        casemapping: isupport::CaseMap,
    },
}

pub fn into_messages(
    broadcast: Broadcast,
    config: &Config,
    sent_time: DateTime<Utc>,
    channels: impl IntoIterator<Item = target::Channel>,
    mut queries: impl IntoIterator<Item = target::Query>
    + std::iter::Iterator<Item = target::Query>,
) -> Vec<Message> {
    match broadcast {
        Broadcast::Connecting => connecting(sent_time),
        Broadcast::Connected => connected(sent_time),
        Broadcast::ConnectionFailed { error } => {
            connection_failed(error, sent_time)
        }
        Broadcast::Disconnected { error } => {
            disconnected(channels, queries, error, sent_time)
        }
        Broadcast::Reconnected => reconnected(channels, queries, sent_time),
        Broadcast::Quit {
            user,
            comment,
            user_channels,
            casemapping,
        } => {
            let user_query = queries.find(|query| {
                user.as_normalized_str() == query.as_normalized_str()
            });

            quit(
                user_channels,
                user_query,
                &user,
                &comment,
                config,
                casemapping,
                sent_time,
            )
        }
        Broadcast::Nickname {
            old_nick,
            new_nick,
            ourself,
            user_channels,
            casemapping,
        } => {
            if ourself {
                // If ourself, broadcast to all query channels (since we are in all of them)
                nickname(
                    user_channels,
                    queries,
                    &old_nick,
                    &new_nick,
                    ourself,
                    casemapping,
                    sent_time,
                )
            } else {
                // Otherwise just the query channel of the user w/ nick change
                let user_query = queries.find(|query| {
                    old_nick.as_normalized_str() == query.as_normalized_str()
                });
                nickname(
                    user_channels,
                    user_query,
                    &old_nick,
                    &new_nick,
                    ourself,
                    casemapping,
                    sent_time,
                )
            }
        }
        Broadcast::Invite {
            inviter,
            channel,
            user_channels,
            casemapping,
        } => invite(inviter, channel, user_channels, casemapping, sent_time),
        Broadcast::ChangeHost {
            old_user,
            new_username,
            new_hostname,
            ourself,
            logged_in,
            user_channels,
            casemapping,
        } => {
            if ourself {
                // If ourself, broadcast to all query channels (since we are in all of them)
                change_host(
                    user_channels,
                    queries,
                    &old_user,
                    &new_username,
                    &new_hostname,
                    ourself,
                    logged_in,
                    casemapping,
                    sent_time,
                )
            } else {
                // Otherwise just the query channel of the user w/ host change
                let user_query = queries.find(|query| {
                    old_user.as_normalized_str() == query.as_normalized_str()
                });
                change_host(
                    user_channels,
                    user_query,
                    &old_user,
                    &new_username,
                    &new_hostname,
                    ourself,
                    logged_in,
                    casemapping,
                    sent_time,
                )
            }
        }
        Broadcast::Kick {
            kicker,
            victim,
            reason,
            channel,
            casemapping,
        } => message::broadcast::kick(
            kicker,
            victim,
            reason,
            channel,
            casemapping,
            sent_time,
        ),
    }
}
