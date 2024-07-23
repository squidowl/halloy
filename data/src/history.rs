use chrono::{DateTime, Utc};
use irc::proto;
use std::path::PathBuf;
use std::time::Duration;
use std::{fmt, io};

use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use tokio::fs;
use tokio::time::Instant;

pub use self::manager::{Manager, Resource};
use crate::user::Nick;
use crate::{compression, environment, isupport, message, server, Message, Server};

pub mod manager;

// TODO: Make this configurable?
/// Max # messages to persist
const MAX_MESSAGES: usize = 10_000;
/// # messages to tuncate after hitting [`MAX_MESSAGES`]
const TRUNC_COUNT: usize = 500;
/// Duration to wait after receiving last message before flushing
const FLUSH_AFTER_LAST_RECEIVED: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    Server,
    Channel(String),
    Query(Nick),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Server => write!(f, "server"),
            Kind::Channel(channel) => write!(f, "channel {channel}"),
            Kind::Query(nick) => write!(f, "user {}", nick),
        }
    }
}

impl From<message::Target> for Kind {
    fn from(target: message::Target) -> Self {
        match target {
            message::Target::Server { .. } => Kind::Server,
            message::Target::Channel { channel, .. } => Kind::Channel(channel),
            message::Target::Query { nick, .. } => Kind::Query(nick),
        }
    }
}

pub async fn load_messages(server: &server::Server, kind: &Kind) -> Vec<Message> {
    let path = messages_path(server, kind);

    if let Ok(messages) = read_messages(&path).await {
        messages
    } else {
        // If messages are not found at the new path, look for messages at the
        // old path.  Old messages stores did not have ordering by server_time
        // strictly enforced, so sort them just in case.

        let hash_path = messages_hash_path(server, kind);

        let mut messages = read_messages(&hash_path).await.unwrap_or_default();

        messages.sort_by(|a, b| a.server_time.cmp(&b.server_time));

        messages
    }
}

pub fn load_read_marker(server: &server::Server, kind: &Kind) -> Option<DateTime<Utc>> {
    let path = read_marker_path(server, kind);

    if let Ok(bytes) = std::fs::read(path) {
        serde_json::from_slice(&bytes).unwrap_or_default()
    } else {
        None
    }
}

pub fn load_targets_marker(server: &server::Server) -> Option<DateTime<Utc>> {
    let path = targets_marker_path(server);

    if let Ok(bytes) = std::fs::read(path) {
        serde_json::from_slice(&bytes).unwrap_or_default()
    } else {
        None
    }
}

pub async fn overwrite(
    server: &server::Server,
    kind: &Kind,
    messages: &[Message],
    read_marker: &Option<DateTime<Utc>>,
) -> Result<(), Error> {
    if messages.is_empty() {
        return Ok(());
    }

    let latest = &messages[messages.len().saturating_sub(MAX_MESSAGES)..];

    let dir = dir_path(server, kind);

    fs::create_dir_all(dir).await?;

    let compressed = compression::compress(&latest)?;

    fs::write(messages_path(server, kind), &compressed).await?;

    let bytes = serde_json::to_vec(&read_marker)?;

    fs::write(read_marker_path(server, kind), &bytes).await?;

    if let Some(read_marker) = read_marker {
        let targets_marker = load_targets_marker(server);

        if !targets_marker.is_some_and(|targets_marker| targets_marker >= *read_marker) {
            let _ = overwrite_targets_marker(server.clone(), *read_marker).await;
        }
    }

    Ok(())
}

pub async fn overwrite_targets_marker(
    server: server::Server,
    targets_marker: DateTime<Utc>,
) -> Result<(), Error> {
    let bytes = serde_json::to_vec(&targets_marker)?;

    fs::write(targets_marker_path(&server), &bytes).await?;

    Ok(())
}

pub async fn append(
    server: &server::Server,
    kind: &Kind,
    messages: Vec<Message>,
    read_marker: &Option<DateTime<Utc>>,
) -> Result<(), Error> {
    if messages.is_empty() {
        return Ok(());
    }

    let mut all_messages = load_messages(server, kind).await;
    messages.into_iter().for_each(|message| {
        insert_message(&mut all_messages, message, &None);
    });

    overwrite(server, kind, &all_messages, read_marker).await
}

async fn read_messages(path: &PathBuf) -> Result<Vec<Message>, Error> {
    let bytes = fs::read(path).await?;
    Ok(compression::decompress(&bytes)?)
}

fn dir_path(server: &server::Server, kind: &Kind) -> PathBuf {
    let data_dir = environment::data_dir();

    let history_dir = data_dir.join("history");

    let server_dir = history_dir.join(format!("{server}"));

    match kind {
        Kind::Server => history_dir,
        Kind::Channel(_) => server_dir,
        Kind::Query(_) => server_dir,
    }
}

fn messages_path(server: &server::Server, kind: &Kind) -> PathBuf {
    let dir = dir_path(server, kind);

    let name = match kind {
        Kind::Server => server.to_string(),
        Kind::Channel(channel) => channel.to_string(),
        Kind::Query(nick) => nick.to_string(),
    };

    dir.join(format!("{name}.json.gz"))
}

fn messages_hash_path(server: &server::Server, kind: &Kind) -> PathBuf {
    let data_dir = environment::data_dir();

    let history_dir = data_dir.join("history");

    let name = match kind {
        Kind::Server => format!("{server}"),
        Kind::Channel(channel) => format!("{server}channel{channel}"),
        Kind::Query(nick) => format!("{server}nickname{}", nick),
    };

    let hashed_name = seahash::hash(name.as_bytes());

    history_dir.join(format!("{hashed_name}.json.gz"))
}

fn read_marker_path(server: &server::Server, kind: &Kind) -> PathBuf {
    let dir = dir_path(server, kind);

    let name = match kind {
        Kind::Server => format!("{server}_read_marker"),
        Kind::Channel(channel) => format!("{channel}_read_marker"),
        Kind::Query(nick) => format!("{}_read_marker", nick),
    };

    dir.join(format!("{name}.json"))
}

fn targets_marker_path(server: &server::Server) -> PathBuf {
    let dir = dir_path(server, &Kind::Server);

    dir.join(format!("{server}_targets_marker.json"))
}

#[derive(Debug)]
pub enum History {
    Partial {
        server: server::Server,
        kind: Kind,
        messages: Vec<Message>,
        last_received_at: Option<Instant>,
        unread_message_count: usize,
        read_marker: Option<DateTime<Utc>>,
    },
    Full {
        server: server::Server,
        kind: Kind,
        messages: Vec<Message>,
        last_received_at: Option<Instant>,
        read_marker: Option<DateTime<Utc>>,
    },
}

impl History {
    fn partial(server: server::Server, kind: Kind, read_marker: Option<DateTime<Utc>>) -> Self {
        Self::Partial {
            server,
            kind,
            messages: vec![],
            last_received_at: None,
            unread_message_count: 0,
            read_marker,
        }
    }

    pub fn inc_unread_count(&mut self) {
        if let History::Partial {
            unread_message_count,
            ..
        } = self
        {
            *unread_message_count += 1;
        }
    }

    fn add_message(&mut self, message: Message) {
        if match self {
            History::Partial {
                messages,
                last_received_at,
                read_marker,
                ..
            }
            | History::Full {
                messages,
                last_received_at,
                read_marker,
                ..
            } => {
                *last_received_at = Some(Instant::now());

                insert_message(messages, message, read_marker)
            }
        } {
            self.inc_unread_count();
        }
    }

    fn flush(&mut self, now: Instant) -> Option<BoxFuture<'static, Result<(), Error>>> {
        match self {
            History::Partial {
                server,
                kind,
                messages,
                read_marker,
                last_received_at,
                ..
            } => {
                if let Some(last_received) = *last_received_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED && !messages.is_empty() {
                        let server = server.clone();
                        let kind = kind.clone();
                        let read_marker = *read_marker;
                        *last_received_at = None;

                        let messages = std::mem::take(messages);

                        return Some(
                            async move { append(&server, &kind, messages, &read_marker).await }
                                .boxed(),
                        );
                    }
                }

                None
            }
            History::Full {
                server,
                kind,
                messages,
                read_marker,
                last_received_at,
                ..
            } => {
                if let Some(last_received) = *last_received_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED && !messages.is_empty() {
                        let server = server.clone();
                        let kind = kind.clone();
                        let read_marker = *read_marker;
                        *last_received_at = None;

                        if messages.len() > MAX_MESSAGES {
                            messages.drain(0..messages.len() - (MAX_MESSAGES - TRUNC_COUNT));
                        }
                        let messages = messages.clone();

                        return Some(
                            async move { overwrite(&server, &kind, &messages, &read_marker).await }
                                .boxed(),
                        );
                    }
                }

                None
            }
        }
    }

    fn make_partial(&mut self) -> Option<impl Future<Output = Result<(), Error>>> {
        match self {
            History::Partial { .. } => None,
            History::Full {
                server,
                kind,
                messages,
                read_marker,
                ..
            } => {
                let server = server.clone();
                let kind = kind.clone();
                let read_marker = messages
                    .iter()
                    .rev()
                    .find(|message| {
                        !matches!(message.target.source(), message::Source::Internal(_))
                    })
                    .map_or(*read_marker, |message| Some(message.server_time));
                let messages = std::mem::take(messages);

                *self = Self::partial(server.clone(), kind.clone(), read_marker);

                Some(async move { overwrite(&server, &kind, &messages, &read_marker).await })
            }
        }
    }

    async fn close(self) -> Result<(), Error> {
        match self {
            History::Partial {
                server,
                kind,
                messages,
                read_marker,
                ..
            } => append(&server, &kind, messages, &read_marker).await,
            History::Full {
                server,
                kind,
                messages,
                read_marker,
                ..
            } => overwrite(&server, &kind, &messages, &read_marker).await,
        }
    }

    fn get_oldest_message(
        &self,
        message_reference_type: &isupport::MessageReferenceType,
    ) -> Option<&Message> {
        match self {
            History::Partial { messages, .. } | History::Full { messages, .. } => messages
                .iter()
                .find(|message| is_referenceable_message(message, Some(message_reference_type))),
        }
    }
}

pub async fn get_latest_message_reference(
    server: Server,
    target: String,
    message_reference_types: Vec<isupport::MessageReferenceType>,
    before_server_time: DateTime<Utc>,
) -> isupport::MessageReference {
    let kind = if proto::is_channel(&target) {
        Kind::Channel(target.clone())
    } else {
        Kind::Query(target.clone().into())
    };

    let messages = load_messages(&server, &kind).await;

    if let Some((latest_message, message_reference_type)) =
        message_reference_types
            .iter()
            .find_map(|message_reference_type| {
                messages
                    .iter()
                    .rev()
                    .find(|message| {
                        message.server_time < before_server_time
                            && is_referenceable_message(message, Some(message_reference_type))
                    })
                    .map(|latest_message| (latest_message, message_reference_type))
            })
    {
        log::debug!("[{server}] {target} - latest_message {:?}", latest_message);
        match message_reference_type {
            isupport::MessageReferenceType::MessageId => {
                if let Some(id) = &latest_message.id {
                    return isupport::MessageReference::MessageId(id.clone());
                }
            }
            isupport::MessageReferenceType::Timestamp => {
                return isupport::MessageReference::Timestamp(latest_message.server_time);
            }
        }
    }

    isupport::MessageReference::None
}

fn is_referenceable_message(
    message: &Message,
    message_reference_type: Option<&isupport::MessageReferenceType>,
) -> bool {
    if matches!(message.target.source(), message::Source::Internal(_)) {
        return false;
    } else if let message::Source::Server(Some(source)) = message.target.source() {
        if matches!(source.kind(), message::source::server::Kind::ReplyTopic) {
            return false;
        }
    }

    if matches!(
        message_reference_type,
        Some(isupport::MessageReferenceType::MessageId)
    ) {
        message.id.is_some()
    } else {
        true
    }
}

pub async fn get_latest_connected_message_reference(
    server: Server,
    before_server_time: DateTime<Utc>,
) -> isupport::MessageReference {
    let messages = load_messages(&server, &Kind::Server).await;

    messages
        .iter()
        .rev()
        .find(|message| {
            message.server_time < before_server_time
                && matches!(
                    message.target.source(),
                    message::Source::Internal(message::source::Internal::Status(
                        message::source::Status::Success
                    ))
                )
        })
        .map_or(isupport::MessageReference::None, |message| {
            isupport::MessageReference::Timestamp(message.server_time)
        })
}

pub async fn num_stored_unread_messages(server: Server, target: String) -> usize {
    let kind = if proto::is_channel(&target) {
        Kind::Channel(target.clone())
    } else {
        Kind::Query(target.clone().into())
    };

    let read_marker = load_read_marker(&server, &kind);

    let messages = load_messages(&server, &kind).await;

    messages
        .into_iter()
        .filter(|message| message.triggers_unread(&read_marker))
        .count()
}

/// Insert the incoming message into the provided vector, sorted
/// on server time
///
/// Deduplication is only checked +/- 1 second around the server time
/// of the incoming message. Either message IDs match, or server times
/// have an exact match + target & content.
pub fn insert_message(
    messages: &mut Vec<Message>,
    message: Message,
    read_marker: &Option<DateTime<Utc>>,
) -> bool {
    let message_triggers_unread = message.triggers_unread(read_marker);

    if messages.is_empty() {
        messages.push(message);

        return message_triggers_unread;
    }

    let start = message::fuzz_start_server_time(message.server_time);
    let end = message::fuzz_end_server_time(message.server_time);

    let start_index = match messages.binary_search_by(|stored| stored.server_time.cmp(&start)) {
        Ok(match_index) => match_index,
        Err(sorted_insert_index) => sorted_insert_index,
    };
    let end_index = match messages.binary_search_by(|stored| stored.server_time.cmp(&end)) {
        Ok(match_index) => match_index,
        Err(sorted_insert_index) => sorted_insert_index,
    };

    let mut current_index = start_index;
    let mut insert_at = start_index;
    let mut replace_at = None;

    for stored in &messages[start_index..end_index] {
        if (message.id.is_some() && stored.id == message.id)
            || ((stored.server_time == message.server_time
                || (matches!(stored.direction, message::Direction::Sent)
                    && matches!(message.direction, message::Direction::Received)))
                && has_matching_content(stored, &message))
        {
            replace_at = Some(current_index);
            break;
        }

        if message.server_time >= stored.server_time {
            insert_at = current_index + 1;
        }

        current_index += 1;
    }

    if let Some(index) = replace_at {
        if has_matching_content(&messages[index], &message) {
            messages[index].id = message.id;
            false
        } else {
            messages[index] = message;
            message_triggers_unread
        }
    } else {
        messages.insert(insert_at, message);
        message_triggers_unread
    }
}

/// The content of JOIN, PART, and QUIT messages may be dependent on how
/// the user attributes are resolved.  Match those messages based on Nick
/// alone (covered by comparing target components) to avoid false negatives.
fn has_matching_content(message: &Message, other: &Message) -> bool {
    if message.target == other.target {
        if let message::Source::Server(Some(source)) = message.target.source() {
            match source.kind() {
                message::source::server::Kind::Join
                | message::source::server::Kind::Part
                | message::source::server::Kind::Quit => {
                    return true;
                }
                message::source::server::Kind::ReplyTopic => (),
            }
        }

        message.content == other.content
    } else {
        false
    }
}

pub fn after_read_marker(message: &Message, read_marker: &Option<DateTime<Utc>>) -> bool {
    read_marker.is_none()
        || read_marker.is_some_and(|read_marker| message.server_time > read_marker)
}

#[derive(Debug)]
pub struct View<'a> {
    pub total: usize,
    pub old_messages: Vec<&'a Message>,
    pub new_messages: Vec<&'a Message>,
    pub max_nick_chars: Option<usize>,
    pub max_prefix_chars: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Compression(#[from] compression::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

#[cfg(test)]
mod test {
    use crate::time::Posix;
    use rand::seq::SliceRandom;

    use super::*;

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn test_insert_message() {
        let mut messages = vec![];

        insert_message(&mut messages, message(1, None, "one"), &None);

        assert_eq!(messages.len(), 1);

        // Insert before single message
        insert_message(&mut messages, message(0, None, "zero"), &None);
        assert_eq!(messages[0].text, "zero".to_string());
        messages.remove(0);

        // Insert after single message
        insert_message(&mut messages, message(2, None, "two"), &None);
        assert_eq!(messages[1].text, "two".to_string());
        messages.remove(1);

        // Insert way before (search slice will be empty)
        insert_message(&mut messages, message(-3_000_000_000, None, "past"), &None);
        assert_eq!(messages[0].text, "past".to_string());
        messages.remove(0);

        // Insert way after (search slice will be empty)
        insert_message(&mut messages, message(3_000_000_000, None, "future"), &None);
        assert_eq!(messages[1].text, "future".to_string());
        messages.remove(1);

        // Insert in random order, assert messages are ordered
        {
            let mut rng = rand::thread_rng();
            let mut tests = (0_i64..10_000).collect::<Vec<_>>();
            tests.shuffle(&mut rng);

            messages.clear();

            for test in tests {
                let millis = test * 1_000;
                insert_message(
                    &mut messages,
                    message(millis, Some(&test.to_string()), millis),
                    &None,
                );
            }

            assert_eq!(messages.len(), 10_000);

            for i in 0usize..10_000 {
                assert_eq!(messages[i].text, (i * 1000).to_string());
            }
        }

        // REPLACE - id match within FUZZ duration (+-1 second)
        for diff in [-999, 0, 999] {
            let millis = 5_000_000 + diff;

            insert_message(
                &mut messages,
                message(millis, Some(&5000.to_string()), diff),
                &None,
            );
            assert_eq!(messages.len(), 10_000);
            assert_eq!(messages[5000].text, diff.to_string());
        }

        // INSERT - id match outside FUZZ duration (1 second)
        for (i, diff) in [-2000, 2000].iter().enumerate() {
            let millis = 5_000_000 + diff;

            insert_message(
                &mut messages,
                message(millis, Some(&5000.to_string()), diff),
                &None,
            );
            assert_eq!(messages.len(), 10_000 + i + 1);
        }
        assert_eq!(messages.len(), 10_002);

        let now = Posix::now();

        // REPLACE - timestamp & content match
        insert_message(&mut messages, message(0, None, 0), &None);
        assert_eq!(messages.len(), 10_002);
        assert!(messages[0].id.is_none());
        assert!(messages[0].received_at >= now);

        // INSERT - timestamp matches but not content
        insert_message(&mut messages, message(0, None, "BAR"), &None);
        assert_eq!(messages.len(), 10_003);
        assert!(messages[1].id.is_none());
        assert_eq!(messages[1].text, "BAR".to_string());
    }

    fn message(millis: i64, id: Option<&str>, text: impl ToString) -> Message {
        Message {
            received_at: Posix::now(),
            server_time: DateTime::from_timestamp_millis(millis).unwrap(),
            direction: message::Direction::Received,
            target: message::Target::Channel {
                channel: "test".to_string(),
                source: message::Source::Server(None),
            },
            text: text.to_string(),
            id: id.map(String::from),
        }
    }
}
