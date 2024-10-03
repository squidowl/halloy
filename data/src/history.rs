use std::path::PathBuf;
use std::time::Duration;
use std::{fmt, io};

use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use irc::proto;
use tokio::fs;
use tokio::time::Instant;

use crate::user::Nick;
use crate::{compression, environment, message, server, Message};

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
    Server,
    Channel(String),
    Query(Nick),
    Logs,
    Highlights,
}

impl Kind {
    pub fn target(&self) -> Option<&str> {
        match self {
            Kind::Server => None,
            Kind::Channel(channel) => Some(channel),
            Kind::Query(nick) => Some(nick.as_ref()),
            Kind::Logs => None,
            Kind::Highlights => None,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Server => write!(f, "server"),
            Kind::Channel(channel) => write!(f, "channel {channel}"),
            Kind::Query(nick) => write!(f, "user {}", nick),
            Kind::Logs => write!(f, "logs"),
            Kind::Highlights => write!(f, "highlights"),
        }
    }
}

impl From<message::Target> for Kind {
    fn from(target: message::Target) -> Self {
        match target {
            message::Target::Server { .. } => Kind::Server,
            message::Target::Channel { channel, .. } => Kind::Channel(channel),
            message::Target::Query { nick, .. } => Kind::Query(nick),
            message::Target::Logs => Kind::Logs,
            message::Target::Highlights { .. } => Kind::Highlights,
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
    read_marker: Option<ReadMarker>,
) -> Result<(), Error> {
    if messages.is_empty() {
        return metadata::save(server, kind, messages, read_marker).await;
    }

    let latest = &messages[messages.len().saturating_sub(MAX_MESSAGES)..];

    let path = path(server, kind).await?;
    let compressed = compression::compress(&latest)?;

    fs::write(path, &compressed).await?;

    metadata::save(server, kind, latest, read_marker).await?;

    Ok(())
}

pub async fn append(
    server: &server::Server,
    kind: &Kind,
    messages: Vec<Message>,
    read_marker: Option<ReadMarker>,
) -> Result<(), Error> {
    let loaded = load(server.clone(), kind.clone()).await?;

    let mut all_messages = loaded.messages;
    all_messages.extend(messages);

    overwrite(server, kind, &all_messages, read_marker).await
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
        Kind::Logs => "logs".to_string(),
        Kind::Highlights => "highlights".to_string(),
    };

    let hashed_name = seahash::hash(name.as_bytes());

    Ok(dir.join(format!("{hashed_name}.json.gz")))
}

#[derive(Debug)]
pub enum History {
    Partial {
        server: server::Server,
        kind: Kind,
        messages: Vec<Message>,
        last_updated_at: Option<Instant>,
        max_triggers_unread: Option<DateTime<Utc>>,
        read_marker: Option<ReadMarker>,
    },
    Full {
        server: server::Server,
        kind: Kind,
        messages: Vec<Message>,
        last_updated_at: Option<Instant>,
        read_marker: Option<ReadMarker>,
    },
}

impl History {
    fn partial(server: server::Server, kind: Kind) -> Self {
        Self::Partial {
            server,
            kind,
            messages: vec![],
            last_updated_at: None,
            max_triggers_unread: None,
            read_marker: None,
        }
    }

    pub fn update_partial(&mut self, metadata: Metadata) {
        if let Self::Partial {
            max_triggers_unread,
            read_marker,
            ..
        } = self
        {
            *read_marker = (*read_marker).max(metadata.read_marker);
            *max_triggers_unread = (*max_triggers_unread).max(metadata.last_triggers_unread);
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
                last_updated_at,
                read_marker,
                ..
            } => {
                if let Some(last_received) = *last_updated_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED {
                        let server = server.clone();
                        let kind = kind.clone();
                        let messages = std::mem::take(messages);
                        let read_marker = *read_marker;

                        *last_updated_at = None;

                        return Some(
                            async move { append(&server, &kind, messages, read_marker).await }
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
                last_updated_at,
                read_marker,
                ..
            } => {
                if let Some(last_received) = *last_updated_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED && !messages.is_empty() {
                        let server = server.clone();
                        let kind = kind.clone();
                        let read_marker = *read_marker;
                        *last_updated_at = None;

                        if messages.len() > MAX_MESSAGES {
                            messages.drain(0..messages.len() - (MAX_MESSAGES - TRUNC_COUNT));
                        }

                        let messages = messages.clone();

                        return Some(
                            async move { overwrite(&server, &kind, &messages, read_marker).await }
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
                read_marker,
                ..
            } => {
                let server = server.clone();
                let kind = kind.clone();
                let messages = std::mem::take(messages);

                let read_marker = ReadMarker::latest(&messages).max(*read_marker);
                let max_triggers_unread = metadata::latest_triggers_unread(&messages);

                *self = Self::Partial {
                    server: server.clone(),
                    kind: kind.clone(),
                    messages: vec![],
                    last_updated_at: None,
                    read_marker,
                    max_triggers_unread,
                };

                Some(async move {
                    overwrite(&server, &kind, &messages, read_marker)
                        .await
                        .map(|_| read_marker)
                })
            }
        }
    }

    async fn close(self) -> Result<Option<ReadMarker>, Error> {
        match self {
            History::Partial {
                server,
                kind,
                messages,
                read_marker,
                ..
            } => {
                append(&server, &kind, messages, read_marker).await?;

                Ok(None)
            }
            History::Full {
                server,
                kind,
                messages,
                read_marker,
                ..
            } => {
                let read_marker = ReadMarker::latest(&messages).max(read_marker);

                overwrite(&server, &kind, &messages, read_marker).await?;

                Ok(read_marker)
            }
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
