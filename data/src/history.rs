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

pub use self::manager::{Manager, ReactionToEcho, Resource};
pub use self::metadata::{Metadata, ReadMarker};
use crate::capabilities::LabeledResponseContext;
use crate::message::{self, Direction, MessageReferences, Source};
use crate::redaction::Redaction;
use crate::target::{self, Target};
use crate::user::Nick;
use crate::{
    Buffer, Message, Server, buffer, compression, config, environment,
    isupport, reaction, redaction,
};

pub mod filter;
pub mod manager;
pub mod metadata;
pub mod reroute;

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
        server: &Server,
        message: &Message,
    ) -> Option<Self> {
        Self::from_server_message_target(server, &message.target)
    }

    pub fn from_server_message_rerouted_from(
        server: &Server,
        message: &Message,
    ) -> Option<Self> {
        message.rerouted_from.as_ref().and_then(|rerouted_from| {
            Self::from_server_message_target(server, rerouted_from)
        })
    }

    fn from_server_message_target(
        server: &Server,
        target: &message::Target,
    ) -> Option<Self> {
        match target {
            message::Target::Server { .. } => {
                Some(Self::Server(server.clone()))
            }
            message::Target::Channel { channel, .. } => {
                Some(Self::Channel(server.clone(), channel.clone()))
            }
            message::Target::Query { query, .. } => {
                Some(Self::Query(server.clone(), query.clone()))
            }
            message::Target::Logs { .. } => None,
            message::Target::Highlights { .. } => None,
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

    pub fn target(&self) -> Option<Target> {
        match self {
            Kind::Server(_) => None,
            Kind::Channel(_, channel) => Some(Target::Channel(channel.clone())),
            Kind::Query(_, nick) => Some(Target::Query(nick.clone())),
            Kind::Logs => None,
            Kind::Highlights => None,
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
        renormalize_messages(messages.iter_mut(), seed);
    }

    let metadata = metadata::load(kind).await.unwrap_or_default();

    Ok(Loaded { messages, metadata })
}

fn renormalize_messages<'a>(
    messages: impl Iterator<Item = &'a mut Message>,
    seed: Seed,
) {
    match seed {
        Seed::Multiple(casemappings) => {
            messages.for_each(|message| {
                if let message::Target::Highlights { server, .. } =
                    &message.target
                    && let Some(casemapping) = casemappings.get(server)
                {
                    message.renormalize(*casemapping);
                }
            });
        }
        Seed::Single(casemapping) => {
            messages.for_each(|message| message.renormalize(casemapping));
        }
    }
}

pub async fn overwrite(
    kind: &Kind,
    messages: &[Message],
    read_marker: Option<ReadMarker>,
    chathistory_references: Option<MessageReferences>,
) -> Result<(), Error> {
    if messages.is_empty() {
        return metadata::save(
            kind,
            messages,
            read_marker,
            chathistory_references,
        )
        .await;
    }

    let latest = &messages[messages.len().saturating_sub(MAX_MESSAGES)..];

    let path = path(kind).await?;
    let compressed = compression::compress(&latest)?;

    fs::write(path, &compressed).await?;

    metadata::save(kind, latest, read_marker, chathistory_references).await?;

    Ok(())
}

pub async fn append(
    kind: &Kind,
    seed: Option<Seed>,
    pending_messages: Vec<(Message, Option<LabeledResponseContext>)>,
    read_marker: Option<ReadMarker>,
    chathistory_references: Option<MessageReferences>,
    pending_reactions: HashMap<message::Id, reaction::Pending>,
    pending_redactions: HashMap<message::Id, redaction::Pending>,
) -> Result<Vec<ReactionToEcho>, Error> {
    let loaded = load(kind.clone(), seed).await?;

    let mut pending_reactions_flushed: Vec<ReactionToEcho> = vec![];

    let mut all_messages = loaded.messages;

    // pending reactions should only exist for unloaded history entries
    for (id, pending) in pending_reactions.into_iter() {
        if let Some(message) =
            find_message_target(&mut all_messages, &id, &pending.server_time)
        {
            if message.is_echo
                && message.direction == Direction::Received
                && let Ok(target) = Target::try_from(message.target.clone())
            {
                let message_text = message.text();
                for (reaction, notification_enabled) in
                    pending.clone().reactions.into_iter()
                {
                    if notification_enabled {
                        let reaction_to_echo = ReactionToEcho {
                            reaction: reaction::Context {
                                inner: reaction,
                                target: target.clone(),
                                in_reply_to: id.clone(),
                                server_time: pending.server_time,
                            },
                            message_text: message_text.clone(),
                        };
                        pending_reactions_flushed.push(reaction_to_echo);
                    }
                }
            }

            message.reactions.append(
                &mut pending
                    .reactions
                    .iter()
                    .map(|(reaction, _)| reaction.clone())
                    .collect(),
            );
        }
    }

    for (id, pending) in pending_redactions.into_iter() {
        if let Some(message) =
            find_message_target(&mut all_messages, &id, &pending.server_time)
        {
            message.redaction = Some(pending.redaction);
        }
    }

    pending_messages.into_iter().for_each(
        |(message, labeled_response_context)| {
            insert_message(
                &mut all_messages,
                message,
                labeled_response_context,
            );
        },
    );

    overwrite(kind, &all_messages, read_marker, chathistory_references)
        .await
        .map(|()| pending_reactions_flushed)
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
    };

    let hashed_name = seahash::hash(name.as_bytes());

    Ok(dir.join(format!("{hashed_name}.json.gz")))
}

#[derive(Debug)]
pub enum History {
    Partial {
        kind: Kind,
        pending_messages: Vec<(Message, Option<LabeledResponseContext>)>, // Unordered
        last_updated_at: Option<Instant>,
        max_triggers_unread: Option<DateTime<Utc>>,
        max_triggers_highlight: Option<DateTime<Utc>>,
        read_marker: Option<ReadMarker>,
        chathistory_references: Option<MessageReferences>,
        last_seen: HashMap<Nick, DateTime<Utc>>,
        pending_reactions: HashMap<message::Id, reaction::Pending>,
        pending_redactions: HashMap<message::Id, redaction::Pending>,
        show_in_sidebar: bool,
    },
    Full {
        kind: Kind,
        messages: Vec<Message>, // Sorted by Message.server_time
        last_updated_at: Option<Instant>,
        read_marker: Option<ReadMarker>,
        display_read_marker: Option<ReadMarker>,
        chathistory_references: Option<MessageReferences>,
        last_seen: HashMap<Nick, DateTime<Utc>>,
        cleared: bool,
    },
}

impl History {
    fn partial(kind: Kind) -> Self {
        Self::Partial {
            kind,
            pending_messages: vec![],
            last_updated_at: None,
            max_triggers_unread: None,
            max_triggers_highlight: None,
            read_marker: None,
            chathistory_references: None,
            last_seen: HashMap::new(),
            pending_reactions: HashMap::new(),
            pending_redactions: HashMap::new(),
            show_in_sidebar: false,
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
            History::Full {
                messages,
                read_marker,
                ..
            } => {
                let latest = metadata::latest_triggers_unread(messages);

                if let Some(read_marker) = read_marker {
                    latest
                        .is_some_and(|latest| read_marker.date_time() < latest)
                } else {
                    latest.is_some()
                }
            }
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
            History::Full {
                messages,
                read_marker,
                ..
            } => {
                let latest = metadata::latest_triggers_highlight(messages);

                if let Some(read_marker) = read_marker {
                    latest
                        .is_some_and(|latest| read_marker.date_time() < latest)
                } else {
                    latest.is_some()
                }
            }
        }
    }

    fn add_message(
        &mut self,
        message: Message,
        labeled_response_context: Option<LabeledResponseContext>,
    ) -> Option<ReadMarker> {
        if let History::Partial {
            show_in_sidebar,
            max_triggers_unread,
            ..
        } = self
            && (matches!(message.direction, message::Direction::Sent)
                || (((message.triggers_unread() && !message.blocked)
                    || (message.is_echo && !message.deduplicate))
                    && Some(message.server_time) > *max_triggers_unread))
        {
            *show_in_sidebar = true;
        }

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
                last_updated_at,
                last_seen,
                ..
            }
            | History::Full {
                last_updated_at,
                last_seen,
                ..
            } => {
                *last_updated_at = Some(Instant::now());

                update_last_seen(last_seen, &message);
            }
        }

        match self {
            History::Partial {
                pending_messages, ..
            } => {
                pending_messages.push((message, labeled_response_context));

                None
            }
            History::Full { messages, .. } => {
                insert_message(messages, message, labeled_response_context)
            }
        }
    }

    fn remove_message(
        &mut self,
        server_time: DateTime<Utc>,
        hash: message::Hash,
    ) -> Option<Message> {
        match self {
            History::Partial {
                pending_messages, ..
            } => pending_messages
                .iter()
                .position(|(message, _)| message.hash == hash)
                .map(|index| {
                    let (message, _) = pending_messages.remove(index);
                    message
                }),
            History::Full { messages, .. } => {
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
                    .position(|message| message.hash == hash)
                    .map(|slice_index| {
                        messages.remove(start_index + slice_index)
                    })
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
            History::Partial { .. } => vec![],
            History::Full { messages, .. } => {
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
    ) -> Option<BoxFuture<'static, Result<Vec<ReactionToEcho>, Error>>> {
        match self {
            History::Partial {
                kind,
                pending_messages,
                last_updated_at,
                read_marker,
                chathistory_references,
                pending_reactions,
                pending_redactions,
                ..
            } => {
                if let Some(last_received) = *last_updated_at
                    && now.is_none_or(|now| {
                        now.duration_since(last_received)
                            >= FLUSH_AFTER_LAST_RECEIVED
                    })
                {
                    let kind = kind.clone();
                    let pending_messages = std::mem::take(pending_messages);
                    let read_marker = *read_marker;
                    let chathistory_references = chathistory_references.clone();
                    let pending_reactions = std::mem::take(pending_reactions);
                    let pending_redactions = std::mem::take(pending_redactions);

                    *last_updated_at = None;

                    return Some(
                        async move {
                            append(
                                &kind,
                                seed,
                                pending_messages,
                                read_marker,
                                chathistory_references,
                                pending_reactions,
                                pending_redactions,
                            )
                            .await
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
                chathistory_references,
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
                    let chathistory_references = chathistory_references.clone();
                    *last_updated_at = None;

                    if messages.len() > MAX_MESSAGES {
                        messages.drain(
                            0..messages.len() - (MAX_MESSAGES - TRUNC_COUNT),
                        );
                    }

                    let messages = messages.clone();

                    return Some(
                        async move {
                            overwrite(
                                &kind,
                                &messages,
                                read_marker,
                                chathistory_references,
                            )
                            .await
                            .map(|()| vec![])
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
                chathistory_references,
                last_seen,
                ..
            } => {
                let kind = kind.clone();
                let last_seen = last_seen.clone();
                let read_marker = *read_marker;
                let max_triggers_unread =
                    metadata::latest_triggers_unread(messages);
                let max_triggers_highlight =
                    metadata::latest_triggers_highlight(messages);
                let chathistory_references =
                    metadata::latest_can_reference(messages)
                        .max(chathistory_references.clone());

                let full_history = std::mem::replace(
                    self,
                    Self::Partial {
                        kind,
                        pending_messages: vec![],
                        last_updated_at: None,
                        read_marker,
                        max_triggers_unread,
                        max_triggers_highlight,
                        chathistory_references: chathistory_references.clone(),
                        last_seen,
                        pending_reactions: HashMap::new(),
                        pending_redactions: HashMap::new(),
                        show_in_sidebar: true,
                    },
                );

                match full_history {
                    History::Partial { .. } => None,
                    History::Full { kind, messages, .. } => Some(async move {
                        overwrite(
                            &kind,
                            &messages,
                            read_marker,
                            chathistory_references,
                        )
                        .await
                    }),
                }
            }
        }
    }

    async fn close(self, seed: Option<Seed>) -> Result<(), Error> {
        match self {
            History::Partial {
                kind,
                pending_messages,
                read_marker,
                chathistory_references,
                pending_reactions,
                pending_redactions,
                ..
            } => append(
                &kind,
                seed,
                pending_messages,
                read_marker,
                chathistory_references,
                pending_reactions,
                pending_redactions,
            )
            .await
            .map(|_| ()),
            History::Full {
                kind,
                messages,
                read_marker,
                chathistory_references,
                ..
            } => {
                overwrite(&kind, &messages, read_marker, chathistory_references)
                    .await
            }
        }
    }

    pub fn mark_as_read(&mut self) -> Option<ReadMarker> {
        let (read_marker, latest) = match self {
            History::Partial {
                max_triggers_unread,
                read_marker,
                ..
            } => (read_marker, max_triggers_unread.map(ReadMarker::from)),
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
        let can_reference = |message: &Message| {
            message.can_reference() && !message.is_rerouted()
        };

        match self {
            History::Partial {
                pending_messages, ..
            } => pending_messages.iter().find_map(|(message, _)| {
                can_reference(message).then_some(message)
            }),
            History::Full { messages, .. } => {
                messages.iter().find(|message| can_reference(message))
            }
        }
    }

    pub fn last_can_reference_before(
        &self,
        server_time: DateTime<Utc>,
    ) -> Option<MessageReferences> {
        let can_reference = |message: &Message| {
            message.can_reference()
                && !message.is_rerouted()
                && message.server_time < server_time
        };

        let (message, chathistory_references) = match self {
            History::Partial {
                pending_messages,
                chathistory_references,
                ..
            } => (
                pending_messages.iter().rev().find_map(|(message, _)| {
                    can_reference(message).then_some(message)
                }),
                chathistory_references,
            ),
            History::Full {
                messages,
                chathistory_references,
                ..
            } => (
                messages.iter().rev().find(|message| can_reference(message)),
                chathistory_references,
            ),
        };

        message.map(Message::references).max(
            if chathistory_references.as_ref().is_some_and(
                |chathistory_references| {
                    chathistory_references.timestamp < server_time
                },
            ) {
                chathistory_references.clone()
            } else {
                None
            },
        )
    }

    pub fn update_chathistory_references(
        &mut self,
        chathistory_references: MessageReferences,
    ) {
        let (stored, last_updated_at) = match self {
            History::Partial {
                chathistory_references: stored_chathistory_references,
                last_updated_at,
                ..
            } => (stored_chathistory_references, last_updated_at),
            History::Full {
                chathistory_references: stored_chathistory_references,
                last_updated_at,
                ..
            } => (stored_chathistory_references, last_updated_at),
        };

        if stored
            .as_ref()
            .is_none_or(|stored| chathistory_references > *stored)
        {
            *stored = Some(chathistory_references);
            *last_updated_at = Some(Instant::now());
        }
    }

    pub fn update_read_marker(&mut self, read_marker: ReadMarker) -> bool {
        let stored = match self {
            History::Partial {
                read_marker: stored_read_marker,
                ..
            } => stored_read_marker,
            History::Full {
                display_read_marker,
                read_marker: stored_read_marker,
                ..
            } => {
                *display_read_marker =
                    (*display_read_marker).max(Some(read_marker));
                stored_read_marker
            }
        };

        if Some(read_marker) > *stored {
            *stored = Some(read_marker);
            true
        } else {
            false
        }
    }

    pub fn read_marker(&self) -> Option<ReadMarker> {
        match self {
            History::Partial { read_marker, .. }
            | History::Full { read_marker, .. } => *read_marker,
        }
    }

    pub fn update_display_read_marker(&mut self, read_marker: ReadMarker) {
        if let History::Full {
            display_read_marker,
            ..
        } = self
        {
            *display_read_marker =
                (*display_read_marker).max(Some(read_marker));
        }
    }

    pub fn display_read_marker(&self) -> Option<ReadMarker> {
        match self {
            History::Partial { .. } => None,
            History::Full {
                display_read_marker,
                ..
            } => *display_read_marker,
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

    pub fn show_preview(&mut self, message: message::Hash, url: &url::Url) {
        if let Self::Full {
            messages,
            last_updated_at,
            ..
        } = self
            && let Some(message) =
                messages.iter_mut().find(|m| m.hash == message)
        {
            message.hidden_urls.remove(url);

            *last_updated_at = Some(Instant::now());
        }
    }

    pub fn add_reaction(
        &mut self,
        reaction: reaction::Context,
        notification_enabled: bool,
    ) -> Option<ReactionToEcho> {
        match self {
            History::Partial {
                pending_messages,
                last_updated_at,
                pending_reactions,
                ..
            } => {
                if let Some(message) =
                    pending_messages.iter_mut().rev().find_map(|(m, _)| {
                        (m.id.as_deref() == Some(&*reaction.in_reply_to))
                            .then_some(m)
                    })
                {
                    let message_text = if message.is_echo
                        && message.direction == Direction::Received
                    {
                        Some(message.text())
                    } else {
                        None
                    };

                    message.reactions.push(reaction.inner.clone());

                    if let Some(message_text) = message_text
                        && notification_enabled
                    {
                        return Some(ReactionToEcho {
                            reaction,
                            message_text,
                        });
                    } else {
                        return None;
                    }
                } else {
                    let pending = pending_reactions
                        .entry(reaction.in_reply_to)
                        .or_insert(reaction::Pending::new(
                            reaction.server_time,
                        ));

                    pending.server_time =
                        (pending.server_time).min(reaction.server_time);
                    pending
                        .reactions
                        .push((reaction.inner, notification_enabled));
                }

                *last_updated_at = Some(Instant::now());
            }
            History::Full {
                messages,
                last_updated_at,
                ..
            } => {
                let message = find_message_target(
                    messages,
                    &reaction.in_reply_to,
                    &reaction.server_time,
                )?;
                message.reactions.push(reaction.inner.clone());

                *last_updated_at = Some(Instant::now());

                if message.is_echo
                    && message.direction == Direction::Received
                    && notification_enabled
                {
                    return Some(ReactionToEcho {
                        reaction,
                        message_text: message.text(),
                    });
                } else {
                    return None;
                };
            }
        }
        None
    }

    pub fn redact_message(
        &mut self,
        id: message::Id,
        redaction: Redaction,
        server_time: DateTime<Utc>,
        display_redacted: bool,
    ) {
        match self {
            History::Partial {
                pending_messages,
                last_updated_at,
                pending_redactions,
                ..
            } => {
                if let Some(message) =
                    pending_messages.iter_mut().rev().find_map(|(m, _)| {
                        (m.id.as_deref() == Some(&*id)).then_some(m)
                    })
                {
                    message.redaction = Some(redaction);
                } else {
                    let pending = pending_redactions.entry(id).or_insert(
                        redaction::Pending::new(redaction, server_time),
                    );

                    pending.server_time =
                        (pending.server_time).min(server_time);
                }

                *last_updated_at = Some(Instant::now());
            }
            History::Full {
                messages,
                last_updated_at,
                ..
            } => {
                let Some(message) =
                    find_message_target(messages, &id, &server_time)
                else {
                    return;
                };

                message.redaction = Some(redaction);

                if !display_redacted {
                    message.blocked = true;
                }

                *last_updated_at = Some(Instant::now());
            }
        }
    }

    pub fn last_seen(&self) -> HashMap<Nick, DateTime<Utc>> {
        match self {
            History::Partial { last_seen, .. }
            | History::Full { last_seen, .. } => last_seen.clone(),
        }
    }

    pub fn renormalize_messages(&mut self, seed: Seed) {
        match self {
            History::Full { messages, .. } => {
                renormalize_messages(messages.iter_mut(), seed);
            }
            History::Partial {
                pending_messages, ..
            } => renormalize_messages(
                pending_messages.iter_mut().map(|(message, _)| message),
                seed,
            ),
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
/// A non-None return value indicates whether a message sent from / this client
/// was replaced by an echo (and the replacement's server_time corresponds to
/// the ReadMarker)
pub fn insert_message(
    messages: &mut Vec<Message>,
    message: Message,
    labeled_response_context: Option<LabeledResponseContext>,
) -> Option<ReadMarker> {
    if messages.is_empty() {
        messages.push(message);

        return None;
    }

    let message_is_unlabeled_echo =
        matches!(message.direction, message::Direction::Received)
            && message.is_echo
            && labeled_response_context.is_none();

    let fuzz_seconds = if message_is_unlabeled_echo {
        chrono::Duration::seconds(300)
    } else {
        chrono::Duration::seconds(1)
    };

    let mut read_marker = None;

    if let Some(labeled_response_context) = &labeled_response_context {
        let start = labeled_response_context.server_time - fuzz_seconds;
        let end = labeled_response_context.server_time + fuzz_seconds;

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
                message
                    .id
                    .as_ref()
                    .is_some_and(|id| {
                        *id == labeled_response_context.label_as_id
                    })
                    .then_some(start_index + slice_index)
            })
        {
            messages.remove(index);

            read_marker = Some(ReadMarker::from(&message));
        }
    }

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

    let mut insert_at = start_index;
    let mut replace_at = None;

    for (current_index, stored) in
        (start_index..).zip(messages[start_index..end_index].iter())
    {
        if replace_at.is_none() && labeled_response_context.is_none() {
            let use_echo_cmp =
                matches!(stored.direction, message::Direction::Sent)
                    && message_is_unlabeled_echo;

            let check_for_matching_content = stored.id.is_none()
                && ((message.id.is_none()
                    && message.deduplicate
                    && stored.server_time == message.server_time)
                    || use_echo_cmp);

            if (message.id.is_some() && stored.id == message.id)
                || (check_for_matching_content
                    && has_matching_content(stored, &message, use_echo_cmp))
            {
                replace_at = Some(current_index);
            }
        }

        if message.server_time >= stored.server_time {
            insert_at = current_index + 1;
        }
    }

    if let Some(index) = replace_at {
        if messages[index].server_time == message.server_time {
            if has_matching_content(&messages[index], &message, false) {
                if let Some(id) = message.id {
                    messages[index].id = Some(id);
                }
                messages[index].received_at = message.received_at;
            } else {
                messages[index] = message;
            }
        } else {
            if message_is_unlabeled_echo {
                read_marker = Some(ReadMarker::from(&message));
            }

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
        }
    } else {
        messages.insert(insert_at, message);
    }

    read_marker
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
                | message::source::server::Kind::Kick
                | message::source::server::Kind::Away
                | message::source::server::Kind::Invite => (),
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

pub fn find_message_target<'a>(
    messages: &'a mut [Message],
    id: &message::Id,
    server_time: &DateTime<Utc>,
) -> Option<&'a mut Message> {
    if messages.is_empty() {
        return None;
    }

    let start = *server_time + chrono::Duration::seconds(1);

    let start_index = match messages
        .binary_search_by(|stored| stored.server_time.cmp(&start))
    {
        Ok(match_index) => match_index,
        Err(sorted_insert_index) => sorted_insert_index,
    };

    // Look for the message at/before the earliest server_time for a react, then
    // check for the unlikely scenario where the message where the message's
    // server_time is after a react
    let position = messages
        .iter()
        .take(start_index)
        .rev()
        .position(|m| m.id.as_deref() == Some(id))
        .map(|position| start_index - 1 - position)
        .or(messages
            .iter()
            .skip(start_index)
            .rev()
            .position(|m| m.id.as_deref() == Some(id))
            .map(|position| messages.len() - 1 - position));

    position.and_then(|position| messages.get_mut(position))
}

#[derive(Debug)]
pub struct View<'a> {
    pub total: usize,
    pub has_more_older_messages: bool,
    pub has_more_newer_messages: bool,
    pub old_messages: Vec<&'a Message>,
    pub new_messages: Vec<&'a Message>,
    pub max_nick_chars: Option<usize>,
    pub max_bot_nick_chars: Option<usize>,
    pub max_prefix_chars: Option<usize>,
    pub range_end_timestamp_chars: Option<usize>,
    pub cleared: bool,
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
