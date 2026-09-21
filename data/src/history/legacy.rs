use std::path::Path;
use std::{fs, io};

use chrono::{DateTime, Utc};
use hashbrown::HashSet;
use serde::Deserialize;

use super::{Id, Kind, Metadata};
use crate::message::{self, Content, MessageReferences, Source, Time};
use crate::serde::{deserialize_date_time_utc_or_epoch, fail_as_none};
use crate::user::Nick;
use crate::{
    Server, command, compression, isupport, reaction, redaction, target,
};

pub fn exists(data_dir: &Path, kind: &Kind) -> Result<bool, io::Error> {
    let (name, metadata_name) = names(kind);
    let development = data_dir.join("msdb");
    let released = data_dir.join("history");
    // A missing archive is normal; inaccessible paths must still fail migration.
    if !name.contains('\0')
        && (development.join(format!("{name}.json.gz")).try_exists()?
            || development
                .join(format!("{metadata_name}.json"))
                .try_exists()?)
    {
        return Ok(true);
    }
    Ok(released
        .join(format!("{}.json.gz", seahash::hash(name.as_bytes())))
        .try_exists()?
        || released
            .join(format!("{}.json", seahash::hash(metadata_name.as_bytes())))
            .try_exists()?)
}

fn names(kind: &Kind) -> (String, String) {
    let name = match kind {
        Kind::Server(server) => format!("{server:b}"),
        Kind::Channel(server, channel) => {
            format!("{server:b}channel{}", channel.as_normalized_str())
        }
        Kind::Query(server, query) => {
            format!("{server:b}nickname{}", query.as_normalized_str())
        }
        Kind::Logs => "logs".into(),
        Kind::Highlights => "highlights".into(),
        Kind::ChannelMonitor => "channel_monitor".into(),
    };
    let metadata_name = match kind {
        Kind::Server(server) => format!("{server}-metadata"),
        Kind::Channel(server, channel) => {
            format!("{server}channel{}-metadata", channel.as_normalized_str())
        }
        Kind::Query(server, query) => {
            format!("{server}nickname{}-metadata", query.as_normalized_str())
        }
        Kind::Logs => "logs-metadata".into(),
        Kind::Highlights => "highlights-metadata".into(),
        Kind::ChannelMonitor => "channel-monitor-metadata".into(),
    };
    (name, metadata_name)
}

pub fn load(
    data_dir: &Path,
    kind: &Kind,
) -> Result<(Vec<message::Message>, Metadata), Error> {
    let (name, metadata_name) = names(kind);
    let development = data_dir.join("msdb");
    // Bouncer identities contain a NUL separator valid only before hashing.
    let (messages, metadata) = if name.contains('\0') {
        (None, None)
    } else {
        (
            read_optional(&development.join(format!("{name}.json.gz")))?,
            read_optional(&development.join(format!("{metadata_name}.json")))?,
        )
    };
    if messages.is_some() || metadata.is_some() {
        let mut messages: Vec<message::Message> = messages
            .map(|bytes| compression::decompress(&bytes))
            .transpose()?
            .unwrap_or_default();
        for message in &mut messages {
            message.history_id = Id::Undetermined;
            if let message::Target::Highlights { history_id, .. }
            | message::Target::ChannelMonitor { history_id, .. } =
                &mut message.target
            {
                *history_id = Id::Undetermined;
            }
        }
        let mut metadata: Metadata = metadata
            .map(|bytes| serde_json::from_slice(&bytes))
            .transpose()?
            .unwrap_or_default();
        // Experimental msdb metadata may contain references from rerouted PMs.
        metadata.latest_chathistory_references = messages
            .iter()
            .filter(|message| {
                !message.is_rerouted()
                    && message.can_reference(&[
                        isupport::MessageReferenceType::MessageId,
                        isupport::MessageReferenceType::Timestamp,
                    ])
            })
            .max_by_key(|message| message.time)
            .map(|message| (message.time, message.references()));
        merge_message_metadata(&mut metadata, &messages);
        return Ok((messages, metadata));
    }
    let released = data_dir.join("history");
    let messages = read_optional(
        &released.join(format!("{}.json.gz", seahash::hash(name.as_bytes()))),
    )?
    .map(|bytes| decode(&bytes))
    .transpose()?
    .unwrap_or_default();
    let metadata: ReleasedMetadata = read_optional(
        &released
            .join(format!("{}.json", seahash::hash(metadata_name.as_bytes()))),
    )?
    .map(|bytes| serde_json::from_slice(&bytes))
    .transpose()?
    .unwrap_or_default();
    let mut metadata = Metadata {
        read_marker: metadata.read_marker,
        latest: messages.iter().map(|message| message.time.utc).max(),
        latest_triggers_unread: metadata.last_triggers_unread,
        latest_triggers_highlight: metadata.last_triggers_highlight,
        latest_chathistory_references: metadata.chathistory_references.map(
            |references| {
                (
                    Time::client(references.timestamp),
                    MessageReferences {
                        timestamp: Some(references.timestamp),
                        id: references.id,
                    },
                )
            },
        ),
    };
    merge_message_metadata(&mut metadata, &messages);
    if let Some(message) = messages
        .iter()
        .filter(|message| {
            !message.is_rerouted()
                && message.can_reference(&[
                    isupport::MessageReferenceType::MessageId,
                    isupport::MessageReferenceType::Timestamp,
                ])
        })
        .max_by_key(|message| message.time)
        && metadata
            .latest_chathistory_references
            .as_ref()
            .is_none_or(|(time, _)| *time < message.time)
    {
        metadata.latest_chathistory_references = Some((
            message.time,
            MessageReferences {
                timestamp: Some(message.time.utc),
                id: message.id.clone(),
            },
        ));
    }
    Ok((messages, metadata))
}

fn merge_message_metadata(
    metadata: &mut Metadata,
    messages: &[message::Message],
) {
    for message in messages {
        metadata.latest = metadata.latest.max(Some(message.time.utc));
        if message.triggers_unread() {
            metadata.latest_triggers_unread =
                metadata.latest_triggers_unread.max(Some(message.time.utc));
        }
        if message.triggers_highlight() {
            metadata.latest_triggers_highlight = metadata
                .latest_triggers_highlight
                .max(Some(message.time.utc));
        }
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, io::Error> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn decode(bytes: &[u8]) -> Result<Vec<message::Message>, Error> {
    let messages: Vec<Message> = compression::decompress(bytes)?;
    Ok(messages.into_iter().map(Message::convert).collect())
}

#[derive(Deserialize, Default)]
struct ReleasedMetadata {
    read_marker: Option<super::ReadMarker>,
    last_triggers_unread: Option<DateTime<Utc>>,
    last_triggers_highlight: Option<DateTime<Utc>>,
    chathistory_references: Option<ReleasedReferences>,
}

#[derive(Deserialize)]
struct ReleasedReferences {
    timestamp: DateTime<Utc>,
    id: Option<message::Id>,
}

#[derive(Deserialize)]
struct Message {
    server_time: DateTime<Utc>,
    direction: Direction,
    target: Target,
    #[serde(default, deserialize_with = "fail_as_none")]
    content: Option<Content>,
    text: Option<String>,
    id: Option<message::Id>,
    #[serde(default, deserialize_with = "fail_as_none")]
    reply_to: Option<message::Id>,
    #[serde(default)]
    hidden_urls: HashSet<url::Url>,
    #[serde(default)]
    is_echo: bool,
    #[serde(default)]
    relayed_by: Option<Nick>,
    #[serde(default)]
    received_with_server_time: bool,
    #[serde(default, deserialize_with = "fail_as_none")]
    command: Option<command::Irc>,
    #[serde(default)]
    reactions: Vec<Reaction>,
    #[serde(default, deserialize_with = "fail_as_none")]
    rerouted_from: Option<Target>,
    #[serde(default, deserialize_with = "fail_as_none")]
    redaction: Option<redaction::Redaction>,
}

impl Message {
    fn convert(self) -> message::Message {
        let (target, source) = self.target.convert();
        message::Message {
            history_id: Id::Undetermined,
            time: Time {
                utc: self.server_time,
                source: if self.received_with_server_time {
                    message::time::Source::Server
                } else {
                    message::time::Source::Client
                },
            },
            direction: match self.direction {
                Direction::Sent => message::Direction::Sent {
                    command: self.command,
                },
                Direction::Received => message::Direction::Received {
                    is_echo: self.is_echo,
                },
            },
            source,
            target,
            content: self.content.unwrap_or_else(|| {
                self.text.map_or_else(
                    || Content::Plain(String::new()),
                    message::parse_fragments,
                )
            }),
            id: self.id,
            hidden_urls: self.hidden_urls,
            reactions: self
                .reactions
                .into_iter()
                .map(|reaction| reaction::Reaction {
                    sender: reaction.sender,
                    text: reaction.text,
                    unreact: reaction.unreact,
                    id: reaction.id,
                    // Released history did not retain reaction timestamp provenance.
                    time: Time::client(reaction.server_time),
                })
                .collect(),
            relayed_by: self.relayed_by,
            rerouted_from: self.rerouted_from.map(|target| target.convert().0),
            redaction: self.redaction,
            reply_to: self.reply_to,
        }
    }
}

#[derive(Deserialize)]
enum Direction {
    Sent,
    Received,
}

#[derive(Deserialize)]
enum Target {
    Server {
        #[serde(deserialize_with = "deserialize_source")]
        source: Source,
    },
    Channel {
        channel: target::Channel,
        #[serde(deserialize_with = "deserialize_source")]
        source: Source,
    },
    Query {
        query: target::Query,
        #[serde(deserialize_with = "deserialize_source")]
        source: Source,
    },
    Logs {
        #[serde(deserialize_with = "deserialize_source")]
        source: Source,
    },
    Highlights {
        server: Server,
        channel: target::Channel,
        #[serde(deserialize_with = "deserialize_source")]
        source: Source,
    },
    ChannelMonitor {
        server: Server,
        channel: target::Channel,
        #[serde(deserialize_with = "deserialize_source")]
        source: Source,
    },
}

impl Target {
    fn convert(self) -> (message::Target, Source) {
        match self {
            Self::Server { source } => (message::Target::Server, source),
            Self::Channel { channel, source } => {
                (message::Target::Channel { channel }, source)
            }
            Self::Query { query, source } => {
                (message::Target::Query { query }, source)
            }
            Self::Logs { source } => (message::Target::Logs, source),
            Self::Highlights {
                server,
                channel,
                source,
            } => (
                message::Target::Highlights {
                    server,
                    channel,
                    history_id: Id::Undetermined,
                },
                source,
            ),
            Self::ChannelMonitor {
                server,
                channel,
                source,
            } => (
                message::Target::ChannelMonitor {
                    server,
                    channel,
                    history_id: Id::Undetermined,
                },
                source,
            ),
        }
    }
}

fn deserialize_source<'de, D>(deserializer: D) -> Result<Source, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum LegacySource {
        User(crate::User),
        Server(Option<LegacyServer>),
        Action(Option<crate::User>),
        Internal(message::source::Internal),
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum LegacyServer {
        Kind(message::source::server::Kind),
        Details(message::source::Server),
    }
    Ok(match LegacySource::deserialize(deserializer)? {
        LegacySource::User(user) => Source::User(user),
        LegacySource::Action(user) => Source::Action(user),
        LegacySource::Internal(internal) => Source::Internal(internal),
        LegacySource::Server(server) => {
            Source::Server(server.map(|server| match server {
                LegacyServer::Kind(kind) => {
                    message::source::Server::new(kind, None, None)
                }
                LegacyServer::Details(details) => details,
            }))
        }
    })
}

#[derive(Deserialize)]
struct Reaction {
    sender: Nick,
    text: String,
    unreact: bool,
    #[serde(default, deserialize_with = "fail_as_none")]
    id: Option<message::Id>,
    #[serde(default, deserialize_with = "deserialize_date_time_utc_or_epoch")]
    server_time: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Compression(#[from] compression::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn released_messages_and_compatibility_fields() {
        let fixture = include_bytes!(
            "../../tests/message/f525f6a90ec5667c6e9b6692ec03da4ce57dbc72.json"
        );
        let mut values: Vec<serde_json::Value> =
            serde_json::from_slice(fixture).unwrap();
        let messages =
            decode(&compression::compress(&values).unwrap()).unwrap();
        assert_eq!(messages.len(), values.len());
        assert!(matches!(messages[0].source, Source::User(_)));
        assert_eq!(messages[0].content.text(), "\\_o< quack!");

        // Synthetic additions cover fields absent from the checked-in fixture.
        let value = &mut values[0];
        value["is_echo"] = true.into();
        value["received_with_server_time"] = true.into();
        value["content"] =
            serde_json::json!({"UnknownFutureContent": "ignored"});
        value["text"] = "see https://halloy.chat".into();
        value["reply_to"] = "original-message".into();
        value["reactions"] = serde_json::json!([
            {"sender": "alice", "text": "👍", "unreact": false,
             "id": "reaction-id", "server_time": "2026-01-02T03:04:05Z"},
            {"sender": "bob", "text": "👍", "unreact": true,
             "id": 42, "server_time": "unusable"}
        ]);
        let converted =
            decode(&compression::compress(&values).unwrap()).unwrap();
        let message = &converted[0];
        assert!(message.is_echo());
        assert!(matches!(message.time.source, message::time::Source::Server));
        assert!(matches!(message.content, Content::Fragments(_)));
        assert_eq!(message.content.text(), "see https://halloy.chat");
        assert_eq!(message.reactions.len(), 2);
        let reaction = &message.reactions[0];
        assert_eq!(reaction.time.utc.to_rfc3339(), "2026-01-02T03:04:05+00:00");
        assert!(matches!(
            reaction.time.source,
            message::time::Source::Client
        ));
        assert_eq!(reaction.text, "👍");
        assert!(reaction.id.is_some());
        assert!(!reaction.unreact);
        assert_eq!(message.reactions[1].time.utc, DateTime::UNIX_EPOCH);
        assert!(message.reactions[1].id.is_none());
        assert!(message.reactions[1].unreact);
        assert!(message.reply_to.is_some());

        let directory = tempfile::tempdir().unwrap();
        let server = Server {
            name: "bouncer".into(),
            network: Some(std::sync::Arc::new(
                crate::bouncer::BouncerNetwork {
                    id: "network-id".into(),
                    name: "network-name".into(),
                },
            )),
        };
        let kind = Kind::Server(server.clone());
        assert!(!exists(directory.path(), &kind).unwrap());
        assert!(load(directory.path(), &kind).unwrap().0.is_empty());
        let released = directory.path().join("history");
        fs::create_dir(&released).unwrap();
        fs::write(
            released.join(format!(
                "{}.json.gz",
                seahash::hash(format!("{server:b}").as_bytes())
            )),
            compression::compress(&values).unwrap(),
        )
        .unwrap();
        assert!(exists(directory.path(), &kind).unwrap());
        assert_eq!(
            load(directory.path(), &kind).unwrap().0.len(),
            values.len()
        );

        let metadata_only = Kind::Highlights;
        let (_, metadata_name) = names(&metadata_only);
        fs::write(
            released.join(format!(
                "{}.json",
                seahash::hash(metadata_name.as_bytes())
            )),
            b"{}",
        )
        .unwrap();
        assert!(exists(directory.path(), &metadata_only).unwrap());
        assert!(load(directory.path(), &metadata_only).unwrap().0.is_empty());
        let invalid_directory = directory.path().join("file");
        fs::write(&invalid_directory, b"").unwrap();
        assert!(exists(&invalid_directory, &metadata_only).is_err());
    }
}
