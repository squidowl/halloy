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
use crate::time::Posix;
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

pub async fn load(server: &server::Server, kind: &Kind) -> Result<Vec<Message>, Error> {
    let path = path(server, kind).await?;

    Ok(read_all(&path).await.unwrap_or_default())
}

pub async fn overwrite(
    server: &server::Server,
    kind: &Kind,
    messages: &[Message],
) -> Result<(), Error> {
    if messages.is_empty() {
        return Ok(());
    }

    let latest = &messages[messages.len().saturating_sub(MAX_MESSAGES)..];

    let path = path(server, kind).await?;
    let compressed = compression::compress(&latest)?;

    fs::write(path, &compressed).await?;

    Ok(())
}

pub async fn append(
    server: &server::Server,
    kind: &Kind,
    messages: Vec<Message>,
) -> Result<(), Error> {
    if messages.is_empty() {
        return Ok(());
    }

    let mut all_messages = load(server, kind).await?;
    all_messages.extend(messages);

    overwrite(server, kind, &all_messages).await
}

async fn read_all(path: &PathBuf) -> Result<Vec<Message>, Error> {
    let bytes = fs::read(path).await?;
    Ok(compression::decompress(&bytes)?)
}

async fn path(server: &server::Server, kind: &Kind) -> Result<PathBuf, Error> {
    let data_dir = environment::data_dir();

    // TODO: Is this stable enough? What if user's nickname changes
    let name = match kind {
        Kind::Server => format!("{server}"),
        Kind::Channel(channel) => format!("{server}channel{channel}"),
        Kind::Query(nick) => format!("{server}nickname{}", nick),
    };
    let hashed_name = seahash::hash(name.as_bytes());

    let parent = data_dir.join("history");

    if !parent.exists() {
        fs::create_dir_all(&parent).await?;
    }

    Ok(parent.join(format!("{hashed_name}.json.gz")))
}

#[derive(Debug)]
pub enum History {
    Partial {
        server: server::Server,
        kind: Kind,
        messages: Vec<Message>,
        last_received_at: Option<Instant>,
        unread_message_count: usize,
        opened_at: Posix,
    },
    Full {
        server: server::Server,
        kind: Kind,
        messages: Vec<Message>,
        last_received_at: Option<Instant>,
        opened_at: Posix,
    },
}

impl History {
    fn partial(server: server::Server, kind: Kind, opened_at: Posix) -> Self {
        Self::Partial {
            server,
            kind,
            messages: vec![],
            last_received_at: None,
            unread_message_count: 0,
            opened_at,
        }
    }

    fn inc_unread_count(&mut self) {
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
                ..
            }
            | History::Full {
                messages,
                last_received_at,
                ..
            } => {
                *last_received_at = Some(Instant::now());

                insert_message(messages, message)
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
                last_received_at,
                ..
            } => {
                if let Some(last_received) = *last_received_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED && !messages.is_empty() {
                        let server = server.clone();
                        let kind = kind.clone();
                        let messages = std::mem::take(messages);
                        *last_received_at = None;

                        return Some(async move { append(&server, &kind, messages).await }.boxed());
                    }
                }

                None
            }
            History::Full {
                server,
                kind,
                messages,
                last_received_at,
                ..
            } => {
                if let Some(last_received) = *last_received_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED && !messages.is_empty() {
                        let server = server.clone();
                        let kind = kind.clone();
                        *last_received_at = None;

                        if messages.len() > MAX_MESSAGES {
                            messages.drain(0..messages.len() - (MAX_MESSAGES - TRUNC_COUNT));
                        }

                        let messages = messages.clone();

                        return Some(
                            async move { overwrite(&server, &kind, &messages).await }.boxed(),
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
                ..
            } => {
                let server = server.clone();
                let kind = kind.clone();
                let messages = std::mem::take(messages);

                *self = Self::partial(server.clone(), kind.clone(), Posix::now());

                Some(async move { overwrite(&server, &kind, &messages).await })
            }
        }
    }

    async fn close(self) -> Result<(), Error> {
        match self {
            History::Partial {
                server,
                kind,
                messages,
                ..
            } => append(&server, &kind, messages).await,
            History::Full {
                server,
                kind,
                messages,
                ..
            } => overwrite(&server, &kind, &messages).await,
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
    join_server_time: DateTime<Utc>,
) -> isupport::MessageReference {
    let kind = if proto::is_channel(&target) {
        Kind::Channel(target.clone())
    } else {
        Kind::Query(target.clone().into())
    };

    if let Ok(messages) = load(&server, &kind).await {
        if let Some((latest_message, message_reference_type)) = message_reference_types
            .iter()
            .find_map(|message_reference_type| {
                messages
                    .iter()
                    .rev()
                    .find(|message| {
                        message
                            .server_time
                            .signed_duration_since(join_server_time)
                            .num_seconds()
                            < 0
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

/// Insert the incoming message into the provided vector, sorted
/// on server time
///
/// Deduplication is only checked +/- 1 second around the server time
/// of the incoming message. Either message IDs match, or server times
/// have an exact match + target & content.
fn insert_message(messages: &mut Vec<Message>, message: Message) -> bool {
    let message_triggers_unread = message.triggers_unread();

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

        message.text == other.text
    } else {
        false
    }
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
}

#[cfg(test)]
mod test {
    use rand::seq::SliceRandom;

    use super::*;

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn test_insert_message() {
        let mut messages = vec![];

        insert_message(&mut messages, message(1, None, "one"));

        assert_eq!(messages.len(), 1);

        // Insert before single message
        insert_message(&mut messages, message(0, None, "zero"));
        assert_eq!(messages[0].text, "zero".to_string());
        messages.remove(0);

        // Insert after single message
        insert_message(&mut messages, message(2, None, "two"));
        assert_eq!(messages[1].text, "two".to_string());
        messages.remove(1);

        // Insert way before (search slice will be empty)
        insert_message(&mut messages, message(-3_000_000_000, None, "past"));
        assert_eq!(messages[0].text, "past".to_string());
        messages.remove(0);

        // Insert way after (search slice will be empty)
        insert_message(&mut messages, message(3_000_000_000, None, "future"));
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
            );
            assert_eq!(messages.len(), 10_000 + i + 1);
        }
        assert_eq!(messages.len(), 10_002);

        let now = Posix::now();

        // REPLACE - timestamp & content match
        insert_message(&mut messages, message(0, None, 0));
        assert_eq!(messages.len(), 10_002);
        assert!(messages[0].id.is_none());
        assert!(messages[0].received_at >= now);

        // INSERT - timestamp matches but not content
        insert_message(&mut messages, message(0, None, "BAR"));
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
