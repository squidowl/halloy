use std::path::PathBuf;
use std::time::Duration;
use std::{fmt, io};

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use irc::proto;
use tokio::fs;
use tokio::time::Instant;

pub use self::manager::{Manager, Resource};
pub use self::metadata::{Metadata, ReadMarker};
use crate::user::Nick;
use crate::{compression, environment, message, server, Message};

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
    Server,
    Channel(String),
    Query(Nick),
}

impl Kind {
    pub fn target(&self) -> Option<&str> {
        match self {
            Kind::Server => None,
            Kind::Channel(channel) => Some(channel),
            Kind::Query(nick) => Some(nick.as_ref()),
        }
    }
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

impl From<String> for Kind {
    fn from(target: String) -> Self {
        Kind::from(target.as_ref())
    }
}

impl From<&str> for Kind {
    fn from(target: &str) -> Self {
        if proto::is_channel(target) {
            Kind::Channel(target.to_string())
        } else {
            Kind::Query(target.to_string().into())
        }
    }
}

#[derive(Debug)]
pub struct Loaded {
    pub messages: Vec<Message>,
    pub metadata: Metadata,
}

pub async fn load(server: server::Server, kind: Kind) -> Result<Loaded, Error> {
    let path = path(&server, &kind).await?;

    let messages = read_all(&path).await.unwrap_or_default();
    let metadata = metadata::load(server, kind).await.unwrap_or_default();

    Ok(Loaded { messages, metadata })
}

pub async fn overwrite(
    server: &server::Server,
    kind: &Kind,
    messages: &[Message],
    metadata: &Metadata,
) -> Result<(), Error> {
    if messages.is_empty() {
        return metadata::save(server, kind, metadata).await;
    }

    let latest = &messages[messages.len().saturating_sub(MAX_MESSAGES)..];

    let path = path(server, kind).await?;
    let compressed = compression::compress(&latest)?;

    fs::write(path, &compressed).await?;

    metadata::save(server, kind, metadata).await?;

    Ok(())
}

pub async fn append(
    server: &server::Server,
    kind: &Kind,
    messages: Vec<Message>,
    metadata: &Metadata,
) -> Result<(), Error> {
    if messages.is_empty() {
        return metadata::save(server, kind, metadata).await;
    }

    let loaded = load(server.clone(), kind.clone()).await?;

    let mut all_messages = loaded.messages;
    all_messages.extend(messages);

    overwrite(server, kind, &all_messages, metadata).await
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

async fn path(server: &server::Server, kind: &Kind) -> Result<PathBuf, Error> {
    let dir = dir_path().await?;

    let name = match kind {
        Kind::Server => format!("{server}"),
        Kind::Channel(channel) => format!("{server}channel{channel}"),
        Kind::Query(nick) => format!("{server}nickname{}", nick),
    };

    let hashed_name = seahash::hash(name.as_bytes());

    Ok(dir.join(format!("{hashed_name}.json.gz")))
}

fn last_in_messages(messages: &[Message]) -> Option<DateTime<Utc>> {
    messages
        .iter()
        .rev()
        .find_map(|message| message.triggers_unread().then_some(message.server_time))
}

#[derive(Debug)]
pub enum History {
    Partial {
        server: server::Server,
        kind: Kind,
        messages: Vec<Message>,
        last_updated_at: Option<Instant>,
        metadata: Metadata,
        last_on_disk: Option<DateTime<Utc>>,
    },
    Full {
        server: server::Server,
        kind: Kind,
        messages: Vec<Message>,
        last_updated_at: Option<Instant>,
        metadata: Metadata,
    },
}

impl History {
    fn partial(server: server::Server, kind: Kind) -> Self {
        Self::Partial {
            server,
            kind,
            messages: vec![],
            last_updated_at: None,
            metadata: Metadata::default(),
            last_on_disk: None,
        }
    }

    pub fn update_partial(&mut self, loaded: Loaded) {
        if let Self::Partial {
            metadata,
            last_on_disk,
            ..
        } = self
        {
            *metadata = metadata.merge(loaded.metadata);
            *last_on_disk = last_in_messages(&loaded.messages).or(*last_on_disk);
        }
    }

    fn has_unread(&self) -> bool {
        match self {
            History::Partial {
                messages,
                metadata,
                last_on_disk,
                ..
            } => {
                // Read marker is prior to last message on disk
                // or prior to unflushed messages in memory
                if let Some(read_marker) = metadata.read_marker {
                    last_on_disk.is_some_and(|last| read_marker.date_time() < last)
                        || messages.iter().any(|message| {
                            read_marker.date_time() < message.server_time
                                && message.triggers_unread()
                        })
                }
                // Default state == unread
                else {
                    true
                }
            }
            History::Full { .. } => false,
        }
    }

    fn add_message(&mut self, message: Message) {
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

                messages.push(message);
            }
        }
    }

    fn flush(&mut self, now: Instant) -> Option<BoxFuture<'static, Result<(), Error>>> {
        match self {
            History::Partial {
                server,
                kind,
                messages,
                metadata,
                last_updated_at,
                last_on_disk,
                ..
            } => {
                if let Some(last_received) = *last_updated_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED {
                        let server = server.clone();
                        let kind = kind.clone();
                        let metadata = *metadata;
                        let messages = std::mem::take(messages);

                        *last_updated_at = None;

                        *last_on_disk = last_in_messages(&messages).or(*last_on_disk);

                        return Some(
                            async move { append(&server, &kind, messages, &metadata).await }
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
                metadata,
                last_updated_at,
                ..
            } => {
                if let Some(last_received) = *last_updated_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED && !messages.is_empty() {
                        let server = server.clone();
                        let kind = kind.clone();
                        let metadata = *metadata;
                        *last_updated_at = None;

                        if messages.len() > MAX_MESSAGES {
                            messages.drain(0..messages.len() - (MAX_MESSAGES - TRUNC_COUNT));
                        }

                        let messages = messages.clone();

                        return Some(
                            async move { overwrite(&server, &kind, &messages, &metadata).await }
                                .boxed(),
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
                server,
                kind,
                messages,
                metadata,
                ..
            } => {
                let server = server.clone();
                let kind = kind.clone();
                let messages = std::mem::take(messages);

                let metadata = metadata.updated(&messages);

                *self = Self::Partial {
                    server: server.clone(),
                    kind: kind.clone(),
                    messages: vec![],
                    last_updated_at: None,
                    metadata,
                    last_on_disk: last_in_messages(&messages),
                };

                Some(async move {
                    overwrite(&server, &kind, &messages, &metadata)
                        .await
                        .map(|_| metadata.read_marker)
                })
            }
        }
    }

    async fn close(self) -> Result<(), Error> {
        match self {
            History::Partial {
                server,
                kind,
                messages,
                metadata,
                ..
            } => {
                let metadata = metadata.updated(&messages);
                append(&server, &kind, messages, &metadata).await
            }
            History::Full {
                server,
                kind,
                messages,
                metadata,
                ..
            } => {
                let metadata = metadata.updated(&messages);
                overwrite(&server, &kind, &messages, &metadata).await
            }
        }
    }

    pub fn update_read_marker(&mut self, read_marker: ReadMarker) {
        let metadata = match self {
            History::Partial { metadata, .. } => metadata,
            History::Full { metadata, .. } => metadata,
        };

        metadata.update_read_marker(read_marker);
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
