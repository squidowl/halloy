//! Generate messages that can be broadcast into every buffer
use chrono::Utc;

use super::{source, Direction, Message, Source, Target};
use crate::time::Posix;
use crate::user::Nick;
use crate::User;

enum Cause {
    Server(source::Server),
    Status(source::Status),
}

fn expand(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
    include_server: bool,
    cause: Cause,
    text: String,
) -> Vec<Message> {
    let message = |target, text| -> Message {
        Message {
            received_at: Posix::now(),
            server_time: Utc::now(),
            direction: Direction::Received,
            target,
            text,
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
                text.clone(),
            )
        })
        .chain(queries.into_iter().map(|nick| {
            message(
                Target::Query {
                    nick,
                    source: source.clone(),
                },
                text.clone(),
            )
        }))
        .chain(include_server.then(|| {
            message(
                Target::Server {
                    source: source.clone(),
                },
                text.clone(),
            )
        }))
        .collect()
}

pub fn connecting() -> Vec<Message> {
    let text = " ∙ connecting to server...".into();
    expand([], [], true, Cause::Status(source::Status::Success), text)
}

pub fn connected() -> Vec<Message> {
    let text = " ∙ connected".into();
    expand([], [], true, Cause::Status(source::Status::Success), text)
}

pub fn connection_failed(error: String) -> Vec<Message> {
    let text = format!(" ∙ connection to server failed ({error})");
    expand([], [], true, Cause::Status(source::Status::Error), text)
}

pub fn disconnected(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
    error: Option<String>,
) -> Vec<Message> {
    let error = error.map(|error| format!(" ({error})")).unwrap_or_default();
    let text = format!(" ∙ connection to server lost{error}");
    expand(
        channels,
        queries,
        true,
        Cause::Status(source::Status::Error),
        text,
    )
}

pub fn reconnected(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
) -> Vec<Message> {
    let text = " ∙ connection to server restored".into();
    expand(
        channels,
        queries,
        true,
        Cause::Status(source::Status::Success),
        text,
    )
}

pub fn quit(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
    user: &User,
    comment: &Option<String>,
) -> Vec<Message> {
    let comment = comment
        .as_ref()
        .map(|comment| format!(" ({comment})"))
        .unwrap_or_default();
    let text = format!("⟵ {} has quit{comment}", user.formatted());

    expand(
        channels,
        queries,
        false,
        Cause::Server(source::Server::Other),
        text,
    )
}

pub fn nickname(
    channels: impl IntoIterator<Item = String>,
    queries: impl IntoIterator<Item = Nick>,
    old_nick: &Nick,
    new_nick: &Nick,
    ourself: bool,
) -> Vec<Message> {
    let text = if ourself {
        format!(" ∙ You're now known as {new_nick}")
    } else {
        format!(" ∙ {old_nick} is now known as {new_nick}")
    };

    expand(
        channels,
        queries,
        false,
        Cause::Server(source::Server::Other),
        text,
    )
}
