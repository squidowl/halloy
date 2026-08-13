use std::fmt;

use crate::client::Destination;
use crate::target::{self, Target};
use crate::{Buffer, Message, Server, buffer, isupport, message};

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
    pub fn server(&self) -> Option<&Server> {
        match self {
            Kind::Server(server) => Some(server),
            Kind::Channel(server, _) => Some(server),
            Kind::Query(server, _) => Some(server),
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
