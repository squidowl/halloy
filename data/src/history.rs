use std::path::PathBuf;
use std::time::Duration;
use std::{fmt, io};

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use tokio::fs;
use tokio::time::Instant;

use crate::message::{self, MessageReferences};
use crate::target::{self, Target};
use crate::{buffer, compression, environment, isupport, Buffer, Message, Server};

pub use self::manager::{Manager, Resource};
pub use self::metadata::{Metadata, ReadMarker};

pub mod manager;
pub mod metadata;

// TODO: Make this configurable?
/// Max # messages to persist
const MAX_MESSAGES: usize = 10_000;
/// # messages to tuncate after hitting [`MAX_MESSAGES`]
const TRUNC_COUNT: usize = 500;
/// Duration to wait after receiving last message before flushing
const FLUSH_AFTER_LAST_RECEIVED: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    Server(Server),
    Channel(Server, target::Channel),
    Query(Server, target::Query),
    Logs,
    Highlights,
}

impl Kind {
    pub fn from_target(server: Server, target: Target) -> Self {
        match target {
            Target::Channel(channel) => Self::Channel(server, channel),
            Target::Query(query) => Self::Query(server, query),
        }
    }

    pub fn from_str(
        server: Server,
        chantypes: &[char],
        statusmsg: &[char],
        casemapping: isupport::CaseMap,
        target: &str,
    ) -> Self {
        Self::from_target(
            server,
            Target::parse(target, chantypes, statusmsg, casemapping),
        )
    }

    pub fn from_input_buffer(buffer: buffer::Upstream) -> Self {
        match buffer {
            buffer::Upstream::Server(server) => Self::Server(server),
            buffer::Upstream::Channel(server, channel) => Self::Channel(server, channel),
            buffer::Upstream::Query(server, nick) => Self::Query(server, nick),
        }
    }

    pub fn from_server_message(server: Server, message: &Message) -> Option<Self> {
        match &message.target {
            message::Target::Server { .. } => Some(Self::Server(server)),
            message::Target::Channel { channel, .. } => {
                Some(Self::Channel(server, channel.clone()))
            }
            message::Target::Query { query, .. } => Some(Self::Query(server, query.clone())),
            message::Target::Logs => None,
            message::Target::Highlights { .. } => None,
        }
    }
}

impl Kind {
    pub fn server(&self) -> Option<&Server> {
        match self {
            Kind::Server(server) => Some(server),
            Kind::Channel(server, _) => Some(server),
            Kind::Query(server, _) => Some(server),
            Kind::Logs => None,
            Kind::Highlights => None,
        }
    }

    pub fn target(&self) -> Option<&str> {
        match self {
            Kind::Server(_) => None,
            Kind::Channel(_, channel) => Some(channel.as_str()),
            Kind::Query(_, nick) => Some(nick.as_str()),
            Kind::Logs => None,
            Kind::Highlights => None,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Server(server) => write!(f, "server on {server}"),
            Kind::Channel(server, channel) => write!(f, "channel {channel} on {server}"),
            Kind::Query(server, nick) => write!(f, "user {nick} on {server}"),
            Kind::Logs => write!(f, "logs"),
            Kind::Highlights => write!(f, "highlights"),
        }
    }
}

impl From<Kind> for Buffer {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Server(server) => Buffer::Upstream(buffer::Upstream::Server(server)),
            Kind::Channel(server, channel) => {
                Buffer::Upstream(buffer::Upstream::Channel(server, channel))
            }
            Kind::Query(server, nick) => Buffer::Upstream(buffer::Upstream::Query(server, nick)),
            Kind::Logs => Buffer::Internal(buffer::Internal::Logs),
            Kind::Highlights => Buffer::Internal(buffer::Internal::Highlights),
        }
    }
}

#[derive(Debug)]
pub struct Loaded {
    pub messages: Vec<Message>,
    pub metadata: Metadata,
}

pub async fn load(kind: Kind) -> Result<Loaded, Error> {
    let path = path(&kind).await?;

    let messages = read_all(&path).await.unwrap_or_default();
    let metadata = metadata::load(kind).await.unwrap_or_default();

    Ok(Loaded { messages, metadata })
}

pub async fn overwrite(
    kind: &Kind,
    messages: &[Message],
    read_marker: Option<ReadMarker>,
) -> Result<(), Error> {
    if messages.is_empty() {
        return metadata::save(kind, messages, read_marker).await;
    }

    let latest = &messages[messages.len().saturating_sub(MAX_MESSAGES)..];

    let path = path(kind).await?;
    let compressed = compression::compress(&latest)?;

    fs::write(path, &compressed).await?;

    metadata::save(kind, latest, read_marker).await?;

    Ok(())
}

pub async fn append(
    kind: &Kind,
    messages: Vec<Message>,
    read_marker: Option<ReadMarker>,
) -> Result<(), Error> {
    let loaded = load(kind.clone()).await?;

    let mut all_messages = loaded.messages;
    messages.into_iter().for_each(|message| {
        insert_message(&mut all_messages, message);
    });

    overwrite(kind, &all_messages, read_marker).await
}

async fn read_all(path: &PathBuf) -> Result<Vec<Message>, Error> {
    let bytes = fs::read(path).await?;
    Ok(compression::decompress(&bytes)?)
}

pub async fn dir_path() -> Result<PathBuf, Error> {
    let data_dir = environment::data_dir();

    let history_dir = data_dir.join("history");

    if !history_dir.exists() {
        fs::create_dir_all(&history_dir).await?;
    }

    Ok(history_dir)
}

async fn path(kind: &Kind) -> Result<PathBuf, Error> {
    let dir = dir_path().await?;

    let name = match kind {
        Kind::Server(server) => format!("{server}"),
        Kind::Channel(server, channel) => format!("{server}channel{}", channel.as_normalized_str()),
        Kind::Query(server, query) => format!("{server}nickname{}", query.as_normalized_str()),
        Kind::Logs => "logs".to_string(),
        Kind::Highlights => "highlights".to_string(),
    };

    let hashed_name = seahash::hash(name.as_bytes());

    Ok(dir.join(format!("{hashed_name}.json.gz")))
}

#[derive(Debug)]
pub enum History {
    Partial {
        kind: Kind,
        messages: Vec<Message>,
        last_updated_at: Option<Instant>,
        max_triggers_unread: Option<DateTime<Utc>>,
        read_marker: Option<ReadMarker>,
        chathistory_references: Option<MessageReferences>,
    },
    Full {
        kind: Kind,
        messages: Vec<Message>,
        last_updated_at: Option<Instant>,
        read_marker: Option<ReadMarker>,
    },
}

impl History {
    fn partial(kind: Kind) -> Self {
        Self::Partial {
            kind,
            messages: vec![],
            last_updated_at: None,
            max_triggers_unread: None,
            read_marker: None,
            chathistory_references: None,
        }
    }

    pub fn update_partial(&mut self, metadata: Metadata) {
        if let Self::Partial {
            max_triggers_unread,
            read_marker,
            chathistory_references,
            ..
        } = self
        {
            *read_marker = (*read_marker).max(metadata.read_marker);
            *max_triggers_unread = (*max_triggers_unread).max(metadata.last_triggers_unread);
            *chathistory_references = chathistory_references
                .clone()
                .max(metadata.chathistory_references);
        }
    }

    fn has_unread(&self) -> bool {
        match self {
            History::Partial {
                max_triggers_unread,
                read_marker,
                ..
            } => {
                // Read marker is prior to last known message which triggers unread
                if let Some(read_marker) = read_marker {
                    max_triggers_unread.is_some_and(|max| read_marker.date_time() < max)
                }
                // Default state == unread if theres messages that trigger indicator
                else {
                    max_triggers_unread.is_some()
                }
            }
            History::Full { .. } => false,
        }
    }

    fn add_message(&mut self, message: Message) {
        if message.triggers_unread() {
            if let History::Partial {
                max_triggers_unread,
                ..
            } = self
            {
                *max_triggers_unread = (*max_triggers_unread).max(Some(message.server_time));
            }
        }

        match self {
            History::Partial {
                messages,
                last_updated_at,
                ..
            }
            | History::Full {
                messages,
                last_updated_at,
                ..
            } => {
                *last_updated_at = Some(Instant::now());

                insert_message(messages, message);
            }
        }
    }

    fn flush(&mut self, now: Instant) -> Option<BoxFuture<'static, Result<(), Error>>> {
        match self {
            History::Partial {
                kind,
                messages,
                last_updated_at,
                read_marker,
                ..
            } => {
                if let Some(last_received) = *last_updated_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED {
                        let kind = kind.clone();
                        let messages = std::mem::take(messages);
                        let read_marker = *read_marker;

                        *last_updated_at = None;

                        return Some(
                            async move { append(&kind, messages, read_marker).await }.boxed(),
                        );
                    }
                }

                None
            }
            History::Full {
                kind,
                messages,
                last_updated_at,
                read_marker,
                ..
            } => {
                if let Some(last_received) = *last_updated_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED && !messages.is_empty() {
                        let kind = kind.clone();
                        let read_marker = *read_marker;
                        *last_updated_at = None;

                        if messages.len() > MAX_MESSAGES {
                            messages.drain(0..messages.len() - (MAX_MESSAGES - TRUNC_COUNT));
                        }

                        let messages = messages.clone();

                        return Some(
                            async move { overwrite(&kind, &messages, read_marker).await }.boxed(),
                        );
                    }
                }

                None
            }
        }
    }

    fn make_partial(&mut self) -> Option<impl Future<Output = Result<Option<ReadMarker>, Error>>> {
        match self {
            History::Partial { .. } => None,
            History::Full {
                kind,
                messages,
                read_marker,
                ..
            } => {
                let kind = kind.clone();
                let messages = std::mem::take(messages);

                let read_marker = ReadMarker::latest(&messages).max(*read_marker);
                let max_triggers_unread = metadata::latest_triggers_unread(&messages);
                let chathistory_references = metadata::latest_can_reference(&messages);

                *self = Self::Partial {
                    kind: kind.clone(),
                    messages: vec![],
                    last_updated_at: None,
                    read_marker,
                    max_triggers_unread,
                    chathistory_references,
                };

                Some(async move {
                    overwrite(&kind, &messages, read_marker)
                        .await
                        .map(|_| read_marker)
                })
            }
        }
    }

    async fn close(self) -> Result<Option<ReadMarker>, Error> {
        match self {
            History::Partial {
                kind,
                messages,
                read_marker,
                ..
            } => {
                append(&kind, messages, read_marker).await?;

                Ok(None)
            }
            History::Full {
                kind,
                messages,
                read_marker,
                ..
            } => {
                let read_marker = ReadMarker::latest(&messages).max(read_marker);

                overwrite(&kind, &messages, read_marker).await?;

                Ok(read_marker)
            }
        }
    }

    pub fn first_can_reference(&self) -> Option<&Message> {
        match self {
            History::Partial { messages, .. } | History::Full { messages, .. } => {
                messages.iter().find(|message| message.can_reference())
            }
        }
    }

    pub fn last_can_reference_before(
        &self,
        server_time: DateTime<Utc>,
    ) -> Option<MessageReferences> {
        match self {
            History::Partial {
                messages,
                chathistory_references,
                ..
            } => messages
                .iter()
                .rev()
                .find(|message| message.can_reference() && message.server_time < server_time)
                .map_or(
                    if chathistory_references
                        .as_ref()
                        .is_some_and(|chathistory_references| {
                            chathistory_references.timestamp < server_time
                        })
                    {
                        chathistory_references.clone()
                    } else {
                        None
                    },
                    |message| Some(message.references()),
                ),
            History::Full { messages, .. } => messages
                .iter()
                .rev()
                .find(|message| message.can_reference() && message.server_time < server_time)
                .map(|message| message.references()),
        }
    }

    pub fn update_read_marker(&mut self, read_marker: ReadMarker) {
        let stored = match self {
            History::Partial { read_marker, .. } => read_marker,
            History::Full { read_marker, .. } => read_marker,
        };

        *stored = (*stored).max(Some(read_marker));
    }

    pub fn read_marker(&self) -> Option<ReadMarker> {
        match self {
            History::Partial { read_marker, .. } | History::Full { read_marker, .. } => {
                *read_marker
            }
        }
    }
}

/// Insert the incoming message into the provided vector, sorted
/// on server time
///
/// Deduplication is only checked +/- 1 second around the server time
/// of the incoming message. Either message IDs match, or server times
/// have an exact match + target & content.
pub fn insert_message(messages: &mut Vec<Message>, message: Message) {
    const FUZZ_SECONDS: chrono::Duration = chrono::Duration::seconds(1);

    if messages.is_empty() {
        messages.push(message);

        return;
    }

    let start = message.server_time - FUZZ_SECONDS;
    let end = message.server_time + FUZZ_SECONDS;

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
            messages[index].received_at = message.received_at;
        } else {
            messages[index] = message;
        }
    } else {
        messages.insert(insert_at, message);
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
                message::source::server::Kind::ReplyTopic
                | message::source::server::Kind::ChangeHost
                | message::source::server::Kind::MonitoredOnline
                | message::source::server::Kind::MonitoredOffline => (),
            }
        }

        message.content == other.content
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
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}
