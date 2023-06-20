use std::path::PathBuf;
use std::time::Duration;
use std::{fmt, io};

use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use tokio::fs;
use tokio::time::Instant;

pub use self::manager::{Manager, Resource};
use crate::user::Nick;
use crate::{compression, message, server, Message};

pub mod manager;

// TODO: Make this configurable?
const MAX_MESSAGES: usize = 10_000;
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

impl From<message::Source> for Kind {
    fn from(source: message::Source) -> Self {
        match source {
            message::Source::Server => Kind::Server,
            message::Source::Channel(channel, _) => Kind::Channel(channel),
            message::Source::Query(user, _) => Kind::Query(user),
        }
    }
}

pub async fn load(server: &server::Name, kind: &Kind) -> Result<Vec<Message>, Error> {
    let path = path(server, kind).await?;

    Ok(read_all(&path).await.unwrap_or_default())
}

pub async fn overwrite(
    server: &server::Name,
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
    server: &server::Name,
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

async fn path(server: &server::Name, kind: &Kind) -> Result<PathBuf, Error> {
    let data_dir = dirs_next::data_dir().ok_or(Error::ResolvableDataDir)?;

    // TODO: Is this stable enough? What if user's nickname changes
    let name = match kind {
        Kind::Server => format!("{server}"),
        Kind::Channel(channel) => format!("{server}channel{channel}"),
        Kind::Query(nick) => format!("{server}nickname{}", nick),
    };
    let hashed_name = seahash::hash(name.as_bytes());

    let parent = data_dir.join("halloy").join("history");

    if !parent.exists() {
        fs::create_dir_all(&parent).await?;
    }

    Ok(parent.join(format!("{hashed_name}")))
}

#[derive(Debug)]
pub enum History {
    Partial {
        server: server::Name,
        kind: Kind,
        messages: Vec<Message>,
        last_received_at: Option<Instant>,
        topic: Option<String>,
    },
    Full {
        server: server::Name,
        kind: Kind,
        messages: Vec<Message>,
        last_received_at: Option<Instant>,
        topic: Option<String>,
    },
}

impl History {
    fn partial(server: server::Name, kind: Kind, topic: Option<String>) -> Self {
        Self::Partial {
            server,
            kind,
            messages: vec![],
            last_received_at: None,
            topic,
        }
    }

    fn messages(&self) -> &[Message] {
        match self {
            History::Partial { messages, .. } => messages,
            History::Full { messages, .. } => messages,
        }
    }

    fn topic(&self) -> Option<&str> {
        match self {
            History::Partial { topic, .. } => topic.as_deref(),
            History::Full { topic, .. } => topic.as_deref(),
        }
    }

    fn add_message(&mut self, message: Message) {
        match self {
            History::Partial {
                messages,
                last_received_at,
                topic,
                ..
            } => {
                if let Some(_topic) = message.content.topic() {
                    *topic = Some(_topic.to_string());
                }

                messages.push(message);
                *last_received_at = Some(Instant::now());
            }
            History::Full {
                messages,
                last_received_at,
                topic,
                ..
            } => {
                if let Some(_topic) = message.content.topic() {
                    *topic = Some(_topic.to_string());
                }

                messages.push(message);
                *last_received_at = Some(Instant::now());
            }
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
                        let messages = messages.clone();
                        *last_received_at = None;

                        return Some(
                            async move { overwrite(&server, &kind, &messages).await }.boxed(),
                        );
                    }
                }

                None
            }
        }
    }

    fn to_partial(&mut self) -> Option<impl Future<Output = Result<(), Error>>> {
        match self {
            History::Partial { .. } => None,
            History::Full {
                server,
                kind,
                messages,
                topic,
                ..
            } => {
                let server = server.clone();
                let kind = kind.clone();
                let messages = std::mem::take(messages);

                *self = Self::partial(server.clone(), kind.clone(), topic.clone());

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
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("can't resolve data directory")]
    ResolvableDataDir,
    #[error(transparent)]
    Compression(#[from] compression::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}
