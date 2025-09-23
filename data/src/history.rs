use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use std::{fmt, io};

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use tokio::fs;
use tokio::time::Instant;

pub use self::manager::{Manager, Resource};
pub use self::metadata::{Metadata, ReadMarker};
use crate::message::{self, MessageReferences, Source};
use crate::target::{self, Target};
use crate::user::Nick;
use crate::{
    Buffer, Message, Server, buffer, compression, config, environment, isupport,
};

pub mod filter;
pub mod manager;
pub mod metadata;

// TODO: Make this configurable?
/// Max # messages to persist
const MAX_MESSAGES: usize = 10_000;
/// # messages to truncate after hitting [`MAX_MESSAGES`]
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
    SearchResults(Server),
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
            buffer::Upstream::Channel(server, channel) => {
                Self::Channel(server, channel)
            }
            buffer::Upstream::Query(server, nick) => Self::Query(server, nick),
        }
    }

    pub fn from_server_message(
        server: Server,
        message: &Message,
    ) -> Option<Self> {
        Self::from_message_target(server, &message.target)
    }

    pub fn from_message_target(
        server: Server,
        message_target: &message::Target,
    ) -> Option<Self> {
        match &message_target {
            message::Target::Server { .. } => Some(Self::Server(server)),
            message::Target::Channel { channel, .. } => {
                Some(Self::Channel(server, channel.clone()))
            }
            message::Target::Query { query, .. } => {
                Some(Self::Query(server, query.clone()))
            }
            message::Target::Logs { .. } => None,
            message::Target::Highlights { .. } => None,
            message::Target::SearchResults { .. } => {
                Some(Self::SearchResults(server))
            }
        }
    }

    pub fn from_buffer(buffer: Buffer) -> Option<Self> {
        match buffer {
            Buffer::Upstream(buffer::Upstream::Server(server)) => {
                Some(Kind::Server(server))
            }
            Buffer::Upstream(buffer::Upstream::Channel(server, channel)) => {
                Some(Kind::Channel(server, channel))
            }
            Buffer::Upstream(buffer::Upstream::Query(server, nick)) => {
                Some(Kind::Query(server, nick))
            }
            Buffer::Internal(buffer::Internal::Logs) => Some(Kind::Logs),
            Buffer::Internal(buffer::Internal::Highlights) => {
                Some(Kind::Highlights)
            }
            Buffer::Internal(buffer::Internal::FileTransfers) => None,
            Buffer::Internal(buffer::Internal::ChannelDiscovery(_)) => None,
            Buffer::Internal(buffer::Internal::SearchResults(server)) => {
                Some(Kind::SearchResults(server))
            }
        }
    }

    pub fn ordered_by(&self) -> OrderedBy {
        match self {
            Kind::SearchResults(_) => OrderedBy::ReceivedAt,
            _ => OrderedBy::default(),
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
            Kind::SearchResults(server) => Some(server),
        }
    }

    pub fn target(&self) -> Option<Target> {
        match self {
            Kind::Server(_) => None,
            Kind::Channel(_, channel) => Some(Target::Channel(channel.clone())),
            Kind::Query(_, nick) => Some(Target::Query(nick.clone())),
            Kind::Logs => None,
            Kind::Highlights => None,
            Kind::SearchResults(_) => None,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Server(server) => write!(f, "server on {server}"),
            Kind::Channel(server, channel) => {
                write!(f, "channel {channel} on {server}")
            }
            Kind::Query(server, nick) => write!(f, "user {nick} on {server}"),
            Kind::Logs => write!(f, "logs"),
            Kind::Highlights => write!(f, "highlights"),
            Kind::SearchResults(server) => {
                write!(f, "search results from {server}")
            }
        }
    }
}

impl From<Kind> for Buffer {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Server(server) => {
                Buffer::Upstream(buffer::Upstream::Server(server))
            }
            Kind::Channel(server, channel) => {
                Buffer::Upstream(buffer::Upstream::Channel(server, channel))
            }
            Kind::Query(server, nick) => {
                Buffer::Upstream(buffer::Upstream::Query(server, nick))
            }
            Kind::Logs => Buffer::Internal(buffer::Internal::Logs),
            Kind::Highlights => Buffer::Internal(buffer::Internal::Highlights),
            Kind::SearchResults(server) => {
                Buffer::Internal(buffer::Internal::SearchResults(server))
            }
        }
    }
}

#[derive(Debug)]
pub struct Loaded {
    pub messages: Vec<Message>,
    pub metadata: Metadata,
}

pub enum Seed {
    Single(isupport::CaseMap),
    Multiple(HashMap<Server, isupport::CaseMap>),
}

pub async fn load(kind: Kind, seed: Option<Seed>) -> Result<Loaded, Error> {
    let path = path(&kind).await?;

    let mut messages = read_all(&path).await.unwrap_or_default();

    if let Some(seed) = seed {
        // TODO: Utilize DeserializeSeed (or equivalent) so proper normalization
        // happens inside read_all, rather than having to renormalize afterward
        renormalize_messages(&mut messages, seed);
    }

    let metadata = metadata::load(kind).await.unwrap_or_default();

    Ok(Loaded { messages, metadata })
}

pub fn renormalize_messages(messages: &mut [Message], seed: Seed) {
    match seed {
        Seed::Multiple(casemappings) => {
            messages.iter_mut().for_each(|message| {
                if let message::Target::Highlights { server, .. } =
                    &message.target
                    && let Some(casemapping) = casemappings.get(server)
                {
                    message.renormalize(*casemapping);
                }
            });
        }
        Seed::Single(casemapping) => {
            messages
                .iter_mut()
                .for_each(|message| message.renormalize(casemapping));
        }
    }
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
    seed: Option<Seed>,
    messages: Vec<Message>,
    read_marker: Option<ReadMarker>,
) -> Result<(), Error> {
    let loaded = load(kind.clone(), seed).await?;

    let mut all_messages = loaded.messages;
    messages.into_iter().for_each(|message| {
        insert_message(&mut all_messages, kind.ordered_by(), message);
    });

    overwrite(kind, &all_messages, read_marker).await
}

pub async fn delete(kind: &Kind) -> Result<(), Error> {
    let path = path(kind).await?;

    fs::remove_file(path).await?;

    Ok(())
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
        Kind::Server(server) => format!("{server:b}"),
        Kind::Channel(server, channel) => {
            format!("{server:b}channel{}", channel.as_normalized_str())
        }
        Kind::Query(server, query) => {
            format!("{server:b}nickname{}", query.as_normalized_str())
        }
        Kind::Logs => "logs".to_string(),
        Kind::Highlights => "highlights".to_string(),
        Kind::SearchResults(server) => {
            format!("{server:b}search results")
        }
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
        max_triggers_highlight: Option<DateTime<Utc>>,
        read_marker: Option<ReadMarker>,
        chathistory_references: Option<MessageReferences>,
        last_seen: HashMap<Nick, DateTime<Utc>>,
    },
    Full {
        kind: Kind,
        messages: Vec<Message>,
        last_updated_at: Option<Instant>,
        read_marker: Option<ReadMarker>,
        last_seen: HashMap<Nick, DateTime<Utc>>,
        cleared: bool,
    },
}

impl History {
    fn partial(kind: Kind) -> Self {
        Self::Partial {
            kind,
            messages: vec![],
            last_updated_at: None,
            max_triggers_unread: None,
            max_triggers_highlight: None,
            read_marker: None,
            chathistory_references: None,
            last_seen: HashMap::new(),
        }
    }

    pub fn update_partial(&mut self, metadata: Metadata) {
        if let Self::Partial {
            max_triggers_unread,
            max_triggers_highlight,
            read_marker,
            chathistory_references,
            ..
        } = self
        {
            *read_marker = (*read_marker).max(metadata.read_marker);
            *max_triggers_unread =
                (*max_triggers_unread).max(metadata.last_triggers_unread);
            *max_triggers_highlight =
                (*max_triggers_highlight).max(metadata.last_triggers_highlight);
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
                    max_triggers_unread
                        .is_some_and(|max| read_marker.date_time() < max)
                }
                // Default state == unread if theres messages that trigger indicator
                else {
                    max_triggers_unread.is_some()
                }
            }
            History::Full { .. } => false,
        }
    }

    fn has_highlight(&self) -> bool {
        match self {
            History::Partial {
                max_triggers_highlight,
                read_marker,
                ..
            } => {
                // Read marker is prior to last known message which triggers highlight
                if let Some(read_marker) = read_marker {
                    max_triggers_highlight
                        .is_some_and(|max| read_marker.date_time() < max)
                }
                // Default state == highlight if theres messages that trigger indicator
                else {
                    max_triggers_highlight.is_some()
                }
            }
            History::Full { .. } => false,
        }
    }

    fn add_message(&mut self, message: Message) -> Option<ReadMarker> {
        if message.triggers_unread()
            && !message.blocked
            && let History::Partial {
                max_triggers_unread,
                ..
            } = self
        {
            *max_triggers_unread =
                (*max_triggers_unread).max(Some(message.server_time));
        }

        if message.triggers_highlight()
            && !message.blocked
            && let History::Partial {
                max_triggers_highlight,
                ..
            } = self
        {
            *max_triggers_highlight =
                (*max_triggers_highlight).max(Some(message.server_time));
        }

        match self {
            History::Partial {
                messages,
                kind,
                last_updated_at,
                last_seen,
                ..
            }
            | History::Full {
                messages,
                kind,
                last_updated_at,
                last_seen,
                ..
            } => {
                *last_updated_at = Some(Instant::now());

                update_last_seen(last_seen, &message);

                insert_message(messages, kind.ordered_by(), message)
            }
        }
    }

    fn remove_message(
        &mut self,
        server_time: DateTime<Utc>,
        hash: message::Hash,
    ) -> Option<Message> {
        match self {
            History::Partial { messages, .. }
            | History::Full { messages, .. } => {
                if messages.is_empty() {
                    return None;
                }

                let fuzz_seconds = chrono::Duration::seconds(1);

                let start = server_time - fuzz_seconds;
                let end = server_time + fuzz_seconds;

                let start_index = match messages
                    .binary_search_by(|stored| stored.server_time.cmp(&start))
                {
                    Ok(match_index) => match_index,
                    Err(sorted_insert_index) => sorted_insert_index,
                };
                let end_index = match messages
                    .binary_search_by(|stored| stored.server_time.cmp(&end))
                {
                    Ok(match_index) => match_index,
                    Err(sorted_insert_index) => sorted_insert_index,
                };

                messages[start_index..end_index]
                    .iter()
                    .enumerate()
                    .find_map(|(slice_index, message)| {
                        (message.hash == hash)
                            .then_some(start_index + slice_index)
                    })
                    .map(|index| messages.remove(index))
            }
        }
    }

    // Find the first message in the condensation, then return all messages in
    // the condensation
    fn get_condensed_messages(
        &mut self,
        server_time: DateTime<Utc>,
        hash: message::Hash,
        config: &config::buffer::Condensation,
    ) -> Vec<&mut Message> {
        match self {
            History::Partial { messages, .. }
            | History::Full { messages, .. } => {
                if messages.is_empty() {
                    return vec![];
                }

                let fuzz_seconds = chrono::Duration::seconds(1);

                let start = server_time - fuzz_seconds;
                let end = server_time + fuzz_seconds;

                let start_index = match messages
                    .binary_search_by(|stored| stored.server_time.cmp(&start))
                {
                    Ok(match_index) => match_index,
                    Err(sorted_insert_index) => sorted_insert_index,
                };
                let end_index = match messages
                    .binary_search_by(|stored| stored.server_time.cmp(&end))
                {
                    Ok(match_index) => match_index,
                    Err(sorted_insert_index) => sorted_insert_index,
                };

                if let Some(index) = messages[start_index..end_index]
                    .iter()
                    .enumerate()
                    .find_map(|(slice_index, message)| {
                        (message.hash == hash)
                            .then_some(start_index + slice_index)
                    })
                    && let Some(first_index) = messages[..=index]
                        .iter()
                        .rev()
                        .position(|message| message.condensed.is_some())
                        .map(|position| index - position)
                {
                    messages[first_index..]
                        .iter_mut()
                        .filter(|message| !message.blocked)
                        .scan(true, |is_first_message, message| {
                            if *is_first_message {
                                *is_first_message = false;
                                Some(message)
                            } else {
                                (message.can_condense(config)
                                    && message.condensed.is_none())
                                .then_some(message)
                            }
                        })
                        .collect()
                } else {
                    vec![]
                }
            }
        }
    }

    // If now is None then history will be flushed regardless of time
    // since last received
    fn flush(
        &mut self,
        now: Option<Instant>,
        seed: Option<Seed>,
    ) -> Option<BoxFuture<'static, Result<(), Error>>> {
        match self {
            History::Partial {
                kind,
                messages,
                last_updated_at,
                read_marker,
                ..
            } => {
                if let Some(last_received) = *last_updated_at
                    && now.is_none_or(|now| {
                        now.duration_since(last_received)
                            >= FLUSH_AFTER_LAST_RECEIVED
                    })
                {
                    let kind = kind.clone();
                    let messages = std::mem::take(messages);
                    let read_marker = *read_marker;

                    *last_updated_at = None;

                    return Some(
                        async move {
                            append(&kind, seed, messages, read_marker).await
                        }
                        .boxed(),
                    );
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
                if let Some(last_received) = *last_updated_at
                    && now.is_none_or(|now| {
                        now.duration_since(last_received)
                            >= FLUSH_AFTER_LAST_RECEIVED
                    })
                    && !messages.is_empty()
                {
                    let kind = kind.clone();
                    let read_marker = *read_marker;
                    *last_updated_at = None;

                    if messages.len() > MAX_MESSAGES {
                        messages.drain(
                            0..messages.len() - (MAX_MESSAGES - TRUNC_COUNT),
                        );
                    }

                    let messages = messages.clone();

                    return Some(
                        async move {
                            overwrite(&kind, &messages, read_marker).await
                        }
                        .boxed(),
                    );
                }

                None
            }
        }
    }

    fn make_partial(
        &mut self,
    ) -> Option<impl Future<Output = Result<(), Error>> + use<>> {
        match self {
            History::Partial { .. } => None,
            History::Full {
                kind,
                messages,
                read_marker,
                last_seen,
                ..
            } => {
                let kind = kind.clone();
                let messages = std::mem::take(messages);
                let read_marker = *read_marker;
                let max_triggers_unread =
                    metadata::latest_triggers_unread(&messages);
                let max_triggers_highlight =
                    metadata::latest_triggers_highlight(&messages);

                let chathistory_references =
                    metadata::latest_can_reference(&messages);

                *self = Self::Partial {
                    kind: kind.clone(),
                    messages: vec![],
                    last_updated_at: None,
                    read_marker,
                    max_triggers_unread,
                    max_triggers_highlight,
                    chathistory_references,
                    last_seen: last_seen.clone(),
                };

                Some(
                    async move { overwrite(&kind, &messages, read_marker).await },
                )
            }
        }
    }

    async fn close(self, seed: Option<Seed>) -> Result<(), Error> {
        match self {
            History::Partial {
                kind,
                messages,
                read_marker,
                ..
            } => append(&kind, seed, messages, read_marker).await,
            History::Full {
                kind,
                messages,
                read_marker,
                ..
            } => overwrite(&kind, &messages, read_marker).await,
        }
    }

    pub fn mark_as_read(&mut self) -> Option<ReadMarker> {
        let (read_marker, latest) = match self {
            History::Partial {
                max_triggers_unread,
                read_marker,
                ..
            } => (
                read_marker,
                max_triggers_unread.map(ReadMarker::from_date_time),
            ),
            History::Full {
                messages,
                read_marker,
                ..
            } => (read_marker, ReadMarker::latest(messages)),
        };

        if latest > *read_marker {
            *read_marker = latest;

            latest
        } else {
            None
        }
    }

    pub fn can_mark_as_read(&self) -> bool {
        match self {
            History::Partial { .. } => self.has_unread(),
            History::Full {
                messages,
                read_marker,
                ..
            } => {
                if messages.is_empty() {
                    false
                } else {
                    *read_marker < ReadMarker::latest(messages)
                }
            }
        }
    }

    pub fn first_can_reference(&self) -> Option<&Message> {
        match self {
            History::Partial { messages, .. }
            | History::Full { messages, .. } => {
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
                .find(|message| {
                    message.can_reference() && message.server_time < server_time
                })
                .map_or(
                    if chathistory_references.as_ref().is_some_and(
                        |chathistory_references| {
                            chathistory_references.timestamp < server_time
                        },
                    ) {
                        chathistory_references.clone()
                    } else {
                        None
                    },
                    |message| Some(message.references()),
                ),
            History::Full { messages, .. } => messages
                .iter()
                .rev()
                .find(|message| {
                    message.can_reference() && message.server_time < server_time
                })
                .map(Message::references),
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
            History::Partial { read_marker, .. }
            | History::Full { read_marker, .. } => *read_marker,
        }
    }

    pub fn hide_preview(&mut self, message: message::Hash, url: url::Url) {
        if let Self::Full {
            messages,
            last_updated_at,
            ..
        } = self
            && let Some(message) =
                messages.iter_mut().find(|m| m.hash == message)
        {
            message.hidden_urls.insert(url);

            *last_updated_at = Some(Instant::now());
        }
    }

    pub fn last_seen(&self) -> HashMap<Nick, DateTime<Utc>> {
        match self {
            History::Partial { last_seen, .. }
            | History::Full { last_seen, .. } => last_seen.clone(),
        }
    }
}

/// Insert the incoming message into the provided vector, sorted
/// on server time
///
/// Deduplication is only checked +/- 1 second around the server time
/// of the incoming message. Either message IDs match, or server times
/// have an exact match + target & content.
///
/// A non-None return value indicates whether a message sent from
/// this client was replaced by an echo
pub fn insert_message(
    messages: &mut Vec<Message>,
    ordered_by: OrderedBy,
    message: Message,
) -> Option<ReadMarker> {
    if messages.is_empty() || matches!(ordered_by, OrderedBy::ReceivedAt) {
        messages.push(message);

        return None;
    }

    let fuzz_seconds =
        if matches!(message.direction, message::Direction::Received)
            && message.is_echo
        {
            chrono::Duration::seconds(300)
        } else {
            chrono::Duration::seconds(1)
        };

    let start = message.server_time - fuzz_seconds;
    let end = message.server_time + fuzz_seconds;

    let start_index = match messages
        .binary_search_by(|stored| stored.server_time.cmp(&start))
    {
        Ok(match_index) => match_index,
        Err(sorted_insert_index) => sorted_insert_index,
    };
    let end_index = match messages
        .binary_search_by(|stored| stored.server_time.cmp(&end))
    {
        Ok(match_index) => match_index,
        Err(sorted_insert_index) => sorted_insert_index,
    };

    let mut current_index = start_index;
    let mut insert_at = start_index;
    let mut replace_at = None;

    for stored in &messages[start_index..end_index] {
        if replace_at.is_none() {
            let use_echo_cmp =
                matches!(stored.direction, message::Direction::Sent)
                    && matches!(
                        message.direction,
                        message::Direction::Received
                    )
                    && message.is_echo;

            if (message.id.is_some() && stored.id == message.id)
                || ((stored.server_time == message.server_time || use_echo_cmp)
                    && has_matching_content(stored, &message, use_echo_cmp))
            {
                replace_at = Some(current_index);
            }
        }

        if message.server_time >= stored.server_time {
            insert_at = current_index + 1;
        }

        current_index += 1;
    }

    if let Some(index) = replace_at {
        if messages[index].server_time == message.server_time {
            if has_matching_content(&messages[index], &message, false) {
                messages[index].id = message.id;
                messages[index].received_at = message.received_at;
            } else {
                messages[index] = message;
            }

            None
        } else {
            let read_marker = if matches!(
                messages[index].direction,
                message::Direction::Sent
            ) && matches!(
                message.direction,
                message::Direction::Received
            ) && message.is_echo
            {
                Some(ReadMarker::from_date_time(message.server_time))
            } else {
                None
            };

            match insert_at.cmp(&index) {
                Ordering::Less => {
                    messages.remove(index);
                    messages.insert(insert_at, message);
                }
                Ordering::Equal => messages[index] = message,
                Ordering::Greater => {
                    messages.insert(insert_at, message);
                    messages.remove(index);
                }
            }

            read_marker
        }
    } else {
        messages.insert(insert_at, message);

        None
    }
}

/// The content of JOIN, PART, and QUIT messages may be dependent on how
/// the user attributes are resolved.  Match those messages based on Nick
/// alone (covered by comparing target components) to avoid false negatives.
fn has_matching_content(
    message: &Message,
    other: &Message,
    use_echo_cmp: bool,
) -> bool {
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
                | message::source::server::Kind::ChangeNick
                | message::source::server::Kind::ChangeMode
                | message::source::server::Kind::ChangeTopic
                | message::source::server::Kind::MonitoredOnline
                | message::source::server::Kind::MonitoredOffline
                | message::source::server::Kind::StandardReply(_)
                | message::source::server::Kind::WAllOps
                | message::source::server::Kind::Kick => (),
            }
        }

        if use_echo_cmp {
            matches!(message.content.echo_cmp(&other.content), Ordering::Equal)
        } else {
            message.content == other.content
        }
    } else {
        false
    }
}

pub fn update_last_seen(
    last_seen: &mut HashMap<Nick, DateTime<Utc>>,
    message: &Message,
) {
    if let Source::User(user) = message.target.source() {
        let nickname = user.nickname().to_owned();

        if let Some(date_time) = last_seen.get_mut(&nickname) {
            if message.server_time > *date_time {
                *date_time = message.server_time;
            }
        } else {
            last_seen.insert(nickname, message.server_time);
        }
    }
}

pub fn get_last_seen(messages: &[Message]) -> HashMap<Nick, DateTime<Utc>> {
    let mut last_seen = HashMap::new();

    messages.iter().for_each(|message| {
        update_last_seen(&mut last_seen, message);
    });

    last_seen
}

#[derive(Debug)]
pub struct View<'a> {
    pub total: usize,
    pub has_more_older_messages: bool,
    pub has_more_newer_messages: bool,
    pub old_messages: Vec<&'a Message>,
    pub new_messages: Vec<&'a Message>,
    pub max_nick_chars: Option<usize>,
    pub max_prefix_chars: Option<usize>,
    pub range_timestamp_extra_chars: Option<usize>,
    pub cleared: bool,
    pub ordered_by: OrderedBy,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum OrderedBy {
    ReceivedAt,
    #[default]
    ServerTime,
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
