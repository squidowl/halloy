use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::time::Duration;
use std::{fmt, io};

use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use irc::proto;
use tokio::fs;
use tokio::time::Instant;

pub use self::manager::{Manager, Resource};
pub use self::metadata::{after_read_marker, Metadata};
use crate::user::Nick;
use crate::{compression, environment, message, server, Message, Server};

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

pub async fn load_messages(server: &server::Server, kind: &Kind) -> Result<Vec<Message>, Error> {
    let path = path(server, kind).await?;

    Ok(read_all(&path).await.unwrap_or_default())
}

pub async fn overwrite(
    server: &server::Server,
    kind: &Kind,
    messages: &[Message],
    metadata: &Metadata,
) -> Result<(), Error> {
    if messages.is_empty() {
        return metadata::overwrite(server, kind, metadata).await;
    }

    let latest = &messages[messages.len().saturating_sub(MAX_MESSAGES)..];

    let path = path(server, kind).await?;
    let compressed = compression::compress(&latest)?;

    fs::write(path, &compressed).await?;

    metadata::overwrite(server, kind, metadata).await?;

    Ok(())
}

pub async fn append(
    server: &server::Server,
    kind: &Kind,
    messages: Vec<Message>,
    metadata: &Metadata,
) -> Result<(), Error> {
    if messages.is_empty() {
        return metadata::overwrite(server, kind, metadata).await;
    }

    let mut all_messages = load_messages(server, kind).await?;
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

#[derive(Debug)]
pub enum History {
    Partial {
        server: server::Server,
        kind: Kind,
        messages: Vec<Message>,
        last_received_at: Option<Instant>,
        unread_message_count: usize,
        metadata: Metadata,
    },
    Full {
        server: server::Server,
        kind: Kind,
        messages: Vec<Message>,
        last_received_at: Option<Instant>,
        metadata: Metadata,
    },
}

impl History {
    fn partial(server: server::Server, kind: Kind, metadata: Metadata) -> Self {
        Self::Partial {
            server,
            kind,
            messages: vec![],
            last_received_at: None,
            unread_message_count: 0,
            metadata,
        }
    }

    pub fn inc_unread_count(&mut self, increment: usize) {
        if let History::Partial {
            unread_message_count,
            ..
        } = self
        {
            *unread_message_count += increment;
        }
    }

    fn add_message(&mut self, message: Message) {
        if match self {
            History::Partial {
                messages,
                last_received_at,
                metadata,
                ..
            }
            | History::Full {
                messages,
                last_received_at,
                metadata,
                ..
            } => {
                *last_received_at = Some(Instant::now());

                insert_message(messages, message, &metadata.read_marker)
            }
        } {
            self.inc_unread_count(1);
        }
    }

    fn flush(&mut self, now: Instant) -> Option<BoxFuture<'static, Result<(), Error>>> {
        match self {
            History::Partial {
                server,
                kind,
                messages,
                metadata,
                last_received_at,
                ..
            } => {
                if let Some(last_received) = *last_received_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED && !messages.is_empty() {
                        let server = server.clone();
                        let kind = kind.clone();
                        let metadata = metadata.clone();
                        let messages = std::mem::take(messages);
                        *last_received_at = None;

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
                last_received_at,
                ..
            } => {
                if let Some(last_received) = *last_received_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_AFTER_LAST_RECEIVED && !messages.is_empty() {
                        let server = server.clone();
                        let kind = kind.clone();
                        let metadata = metadata.clone();
                        *last_received_at = None;

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

    fn make_partial(&mut self) -> Option<impl Future<Output = Result<(), Error>>> {
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
                let read_marker = messages
                    .iter()
                    .rev()
                    .find(|message| {
                        !matches!(message.target.source(), message::Source::Internal(_))
                    })
                    .map_or(metadata.read_marker, |message| Some(message.server_time));
                let metadata = Metadata { read_marker };
                let messages = std::mem::take(messages);

                *self = Self::partial(server.clone(), kind.clone(), metadata.clone());

                Some(async move { overwrite(&server, &kind, &messages, &metadata).await })
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
            } => append(&server, &kind, messages, &metadata).await,
            History::Full {
                server,
                kind,
                messages,
                metadata,
                ..
            } => overwrite(&server, &kind, &messages, &metadata).await,
        }
    }

    pub fn get_read_marker(&self) -> Option<DateTime<Utc>> {
        match self {
            History::Partial { metadata, .. } => metadata.read_marker,
            History::Full { metadata, .. } => metadata.read_marker,
        }
    }

    pub fn update_read_marker(&mut self, read_marker: Option<DateTime<Utc>>) -> bool {
        let metadata = match self {
            History::Partial { metadata, .. } => metadata,
            History::Full { metadata, .. } => metadata,
        };

        if let Some(read_marker) = read_marker {
            if let Some(history_read_marker) = metadata.read_marker {
                if read_marker <= history_read_marker {
                    return false;
                }
            }

            metadata.read_marker = Some(read_marker);
        } else {
            return false;
        }

        if let History::Partial {
            messages,
            unread_message_count,
            metadata,
            ..
        } = self
        {
            *unread_message_count = 0;

            for message in messages {
                if message.triggers_unread(&metadata.read_marker) {
                    *unread_message_count += 1;
                }
            }
        }

        true
    }
}

pub fn insert_message(
    messages: &mut Vec<Message>,
    message: Message,
    read_marker: &Option<DateTime<Utc>>,
) -> bool {
    let message_triggers_unread = message.triggers_unread(read_marker);

    messages.push(message);

    message_triggers_unread
}

pub async fn num_stored_unread_messages(
    server: Server,
    kind: Kind,
    read_marker: Option<DateTime<Utc>>,
) -> usize {
    let messages = load_messages(&server, &kind).await;

    if let Ok(messages) = messages {
        messages
            .into_iter()
            .rev()
            .map_while(|message| {
                if after_read_marker(&message, &read_marker) {
                    Some(message)
                } else {
                    None
                }
            })
            .filter(|message| message.triggers_unread(&read_marker))
            .count()
    } else {
        0
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
