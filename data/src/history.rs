use std::collections::HashMap;
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
use crate::{compression, environment, message, server, Buffer, Message};

pub mod manager;

// TODO: Make this configurable?
/// Max # messages to persist
const MAX_MESSAGES: usize = 10_000;
/// # messages to tuncate after hitting [`MAX_MESSAGES`]
const TRUNC_COUNT: usize = 500;
/// Duration to wait after receiving last message before flushing
const FLUSH_AFTER_LAST_RECEIVED: Duration = Duration::from_secs(5);
const INPUT_HISTORY_LENGTH: usize = 100;

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

    fn add_message(&mut self, message: Message) {
        match self {
            History::Partial {
                messages,
                last_received_at,
                unread_message_count,
                ..
            } => {
                if message.triggers_unread() {
                    *unread_message_count += 1;
                }

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
}

#[derive(Debug)]
pub struct View<'a> {
    pub total: usize,
    pub old_messages: Vec<&'a Message>,
    pub new_messages: Vec<&'a Message>,
}

#[derive(Debug, Clone, Default)]
struct Input(HashMap<Buffer, Vec<String>>);

impl Input {
    fn get<'a>(&'a self, buffer: &Buffer) -> &'a [String] {
        self.0.get(buffer).map(Vec::as_slice).unwrap_or_default()
    }

    fn push(&mut self, buffer: &Buffer, text: String) {
        let history = self.0.entry(buffer.clone()).or_default();
        history.insert(0, text);
        history.truncate(INPUT_HISTORY_LENGTH);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Compression(#[from] compression::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}
