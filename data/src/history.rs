use std::path::PathBuf;
use std::time::Duration;
use std::{fmt, io};

use futures::future::BoxFuture;
use futures::{Future, FutureExt};
use tokio::fs;
use tokio::time::Instant;

pub use self::manager::{Manager, Resource};
use crate::{compression, message, server, Message, User};

pub mod manager;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    Server,
    Channel(String),
    Query(User),
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Server => write!(f, "server"),
            Kind::Channel(channel) => write!(f, "channel {channel}"),
            Kind::Query(user) => write!(f, "user {}", user.nickname()),
        }
    }
}

impl From<message::Source> for Kind {
    fn from(source: message::Source) -> Self {
        match source {
            message::Source::Server => Kind::Server,
            message::Source::Channel(channel, _) => Kind::Channel(channel),
            message::Source::Query(user) => Kind::Query(user),
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

    let path = path(server, kind).await?;
    let compressed = compression::compress(&messages)?;

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
        Kind::Query(user) => format!("{server}nickname{}", user.nickname()),
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
    },
    Full {
        server: server::Name,
        kind: Kind,
        messages: Vec<Message>,
        last_received_at: Option<Instant>,
    },
}

impl History {
    fn partial(server: server::Name, kind: Kind) -> Self {
        Self::Partial {
            server,
            kind,
            messages: vec![],
            last_received_at: None,
        }
    }

    fn messages(&self) -> &[Message] {
        match self {
            History::Partial { messages, .. } => messages,
            History::Full { messages, .. } => messages,
        }
    }

    fn add_message(&mut self, message: Message) {
        match self {
            History::Partial {
                messages,
                last_received_at,
                ..
            } => {
                messages.push(message);
                *last_received_at = Some(Instant::now());
            }
            History::Full {
                messages,
                last_received_at,
                ..
            } => {
                messages.push(message);
                *last_received_at = Some(Instant::now());
            }
        }
    }

    fn flush(&mut self, now: Instant) -> Option<BoxFuture<'static, Result<(), Error>>> {
        const FLUSH_DURATION: Duration = Duration::from_secs(3);

        match self {
            History::Partial {
                server,
                kind,
                messages,
                last_received_at,
            } => {
                if let Some(last_received) = *last_received_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_DURATION && !messages.is_empty() {
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
            } => {
                if let Some(last_received) = *last_received_at {
                    let since = now.duration_since(last_received);

                    if since >= FLUSH_DURATION && !messages.is_empty() {
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

    fn close(&mut self) -> Option<impl Future<Output = Result<(), Error>>> {
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

                *self = History::partial(server.clone(), kind.clone());

                Some(async move { overwrite(&server, &kind, &messages).await })
            }
        }
    }

    async fn exit(self) -> Result<(), Error> {
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
