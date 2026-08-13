use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use self::metadata::{Metadata, ReadMarker};
pub use self::model::Model;
pub use self::storage::Storage;
use crate::client::Destination;
use crate::target::{self, Target, TargetRef};
use crate::{Buffer, Message, Server, buffer, isupport, message};

pub mod filter;
pub mod metadata;
pub mod model;
pub mod reroute;
pub mod storage;

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum Id {
    #[default]
    Undetermined,
    Determined(u64),
}

#[derive(Debug)]
pub struct Request {
    pub limit: message::Limit,
    pub clear: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    Server(Server),
    Channel(Server, target::Channel),
    Query(Server, target::Query),
    Logs,
    Highlights,
    ChannelMonitor,
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

    pub fn from_message(
        message: &Message,
        server: Option<&Server>,
    ) -> Option<Self> {
        match &message.target {
            message::Target::Server => {
                server.map(|server| Self::Server(server.clone()))
            }
            message::Target::Channel { channel } => server
                .map(|server| Self::Channel(server.clone(), channel.clone())),
            message::Target::Query { query } => {
                server.map(|server| Self::Query(server.clone(), query.clone()))
            }
            message::Target::Logs => Some(Self::Logs),
            message::Target::Highlights { .. } => Some(Self::Highlights),
            message::Target::ChannelMonitor { .. } => {
                Some(Self::ChannelMonitor)
            }
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
            message::Target::Server => Some(Self::Server(server.clone())),
            message::Target::Channel { channel } => {
                Some(Self::Channel(server.clone(), channel.clone()))
            }
            message::Target::Query { query } => {
                Some(Self::Query(server.clone(), query.clone()))
            }
            message::Target::Logs => None,
            message::Target::Highlights { .. } => None,
            message::Target::ChannelMonitor { .. } => None,
        }
    }

    pub fn from_server_destination(
        server: Server,
        destination: Destination,
    ) -> Self {
        match destination {
            Destination::Server => Kind::Server(server),
            Destination::Target(target) => Kind::from_target(server, target),
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
            Buffer::Internal(buffer::Internal::ChannelMonitor) => {
                Some(Kind::ChannelMonitor)
            }
            Buffer::Internal(buffer::Internal::ChannelDiscovery(_)) => None,
            Buffer::Internal(buffer::Internal::ConfigEditor) => None,
        }
    }
}

impl Kind {
    pub fn as_server(&self) -> Option<&Server> {
        match self {
            Kind::Server(server) => Some(server),
            Kind::Channel(server, _) => Some(server),
            Kind::Query(server, _) => Some(server),
            Kind::Logs => None,
            Kind::Highlights => None,
            Kind::ChannelMonitor => None,
        }
    }

    pub fn as_targetref(&self) -> Option<TargetRef<'_>> {
        match self {
            Kind::Server(_) => None,
            Kind::Channel(_, channel) => Some(TargetRef::Channel(channel)),
            Kind::Query(_, nick) => Some(TargetRef::Query(nick)),
            Kind::Logs => None,
            Kind::Highlights => None,
            Kind::ChannelMonitor => None,
        }
    }

    pub fn target(&self) -> Option<Target> {
        match self {
            Kind::Server(_) => None,
            Kind::Channel(_, channel) => Some(Target::Channel(channel.clone())),
            Kind::Query(_, nick) => Some(Target::Query(nick.clone())),
            Kind::Logs => None,
            Kind::Highlights => None,
            Kind::ChannelMonitor => None,
        }
    }

    pub fn as_channel(&self) -> Option<&target::Channel> {
        match self {
            Kind::Server(_) => None,
            Kind::Channel(_, channel) => Some(channel),
            Kind::Query(_, _) => None,
            Kind::Logs => None,
            Kind::Highlights => None,
            Kind::ChannelMonitor => None,
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
            Kind::ChannelMonitor => write!(f, "channel monitor"),
        }
    }
}

impl From<buffer::Upstream> for Kind {
    fn from(upstream_buffer: buffer::Upstream) -> Self {
        match upstream_buffer {
            buffer::Upstream::Server(server) => Self::Server(server),
            buffer::Upstream::Channel(server, channel) => {
                Self::Channel(server, channel)
            }
            buffer::Upstream::Query(server, nick) => Self::Query(server, nick),
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
            Kind::ChannelMonitor => {
                Buffer::Internal(buffer::Internal::ChannelMonitor)
            }
        }
    }
}

pub fn smart_filter_message<M>(
    message: &M,
    seconds: &i64,
    last_seen: Option<&DateTime<Utc>>,
) -> bool
where
    M: message::Temporal,
{
    let Some(last_seen) = last_seen else {
        return true;
    };

    let duration_seconds = message
        .time()
        .utc
        .signed_duration_since(*last_seen)
        .num_seconds();

    duration_seconds > *seconds
}

pub fn smart_filter_repeat<M>(
    message: &M,
    seconds: &i64,
    last_seen: Option<&DateTime<Utc>>,
) -> bool
where
    M: message::Temporal,
{
    let Some(last_seen) = last_seen else {
        return false;
    };

    let duration_seconds = message
        .time()
        .utc
        .signed_duration_since(*last_seen)
        .num_seconds();

    duration_seconds <= *seconds
}

pub fn smart_filter_internal_message<M>(
    message: &M,
    seconds: &i64,
    current_time: &DateTime<Utc>,
) -> bool
where
    M: message::Temporal,
{
    let duration_seconds = current_time
        .signed_duration_since(message.time().utc)
        .num_seconds();

    duration_seconds > *seconds
}

pub fn find_message_by_history_id<'a, M>(
    messages: &'a [M],
    history_id: &Id,
    time: &message::Time,
) -> Option<&'a M>
where
    M: message::Searchable,
{
    position_message_by_history_id(messages, history_id, time)
        .and_then(|position| messages.get(position))
}

pub fn find_message_mut_by_history_id<'a, M>(
    messages: &'a mut [M],
    history_id: &Id,
    time: &message::Time,
) -> Option<&'a mut M>
where
    M: message::Searchable,
{
    position_message_by_history_id(messages, history_id, time)
        .and_then(|position| messages.get_mut(position))
}

pub fn position_message_by_history_id<M>(
    messages: &[M],
    history_id: &Id,
    time: &message::Time,
) -> Option<usize>
where
    M: message::Searchable,
{
    position_message(
        messages,
        |message| message.history_id() == history_id,
        time,
    )
}

pub fn find_message_by_id<'a, M>(
    messages: &'a [M],
    id: &message::Id,
    time: &message::Time,
) -> Option<&'a M>
where
    M: message::Searchable,
{
    position_message_by_id(messages, id, time)
        .and_then(|position| messages.get(position))
}

pub fn find_message_mut_by_id<'a, M>(
    messages: &'a mut [M],
    id: &message::Id,
    time: &message::Time,
) -> Option<&'a mut M>
where
    M: message::Searchable,
{
    position_message_by_id(messages, id, time)
        .and_then(|position| messages.get_mut(position))
}

pub fn position_message_by_id<M>(
    messages: &[M],
    id: &message::Id,
    time: &message::Time,
) -> Option<usize>
where
    M: message::Searchable,
{
    position_message(
        messages,
        |message| message.id().as_deref() == Some(id),
        time,
    )
}

pub fn position_message<M>(
    messages: &[M],
    is_match: impl Fn(&M) -> bool,
    time: &message::Time,
) -> Option<usize>
where
    M: message::Searchable,
{
    if messages.is_empty() {
        return None;
    }

    // We're either looking for the message at time or one that is expected to
    // before (e.g. the message that is reacted or replied to at time).  Fuzz
    // ahead one second to ensure all messages at time are checked (without
    // having to find the exact last index with the input time).

    let start = time.utc + chrono::Duration::seconds(1);

    let start_index = match messages
        .binary_search_by(|stored| stored.time().utc.cmp(&start))
    {
        Ok(match_index) => match_index,
        Err(sorted_insert_index) => sorted_insert_index,
    };

    // Check messages at time, then before time, then check for the unlikely
    // scenario where the message we're looking for is after the provided time.

    messages
        .iter()
        .take(start_index)
        .rev()
        .position(&is_match)
        .map(|position| start_index - 1 - position)
        .or(messages
            .iter()
            .skip(start_index)
            .rev()
            .position(is_match)
            .map(|position| messages.len() - 1 - position))
}

pub fn position_message_after_date_time<M>(
    messages: &[M],
    date_time: &DateTime<Utc>,
) -> usize
where
    M: message::Searchable,
{
    let start_index = match messages
        .binary_search_by(|message| message.time().utc.cmp(date_time))
    {
        Ok(match_index) => match_index,
        Err(sorted_insert_index) => {
            return sorted_insert_index;
        }
    };

    start_index.saturating_add(
        messages[start_index..]
            .iter()
            .position(|message| message.time().utc > *date_time)
            .unwrap_or(1),
    )
}
