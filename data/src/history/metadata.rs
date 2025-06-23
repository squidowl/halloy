use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::format::SecondsFormat;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::Message;
use crate::history::{Error, Kind, dir_path};
use crate::message::{MessageReferences, source};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Metadata {
    pub read_marker: Option<ReadMarker>,
    pub last_triggers_unread: Option<DateTime<Utc>>,
    pub chathistory_references: Option<MessageReferences>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Deserialize,
    Serialize,
)]
pub struct ReadMarker(DateTime<Utc>);

impl ReadMarker {
    pub fn from_date_time(date_time: DateTime<Utc>) -> Self {
        ReadMarker(date_time)
    }

    pub fn latest(messages: &[Message]) -> Option<Self> {
        messages
            .iter()
            .rev()
            .find(|message| match message.target.source() {
                source::Source::Internal(source) => match source {
                    source::Internal::Status(_) => false,
                    // Logs are in their own buffer and this gives us backlog support there
                    source::Internal::Logs => true,
                },
                _ => true,
            })
            .map(|message| message.server_time)
            .map(Self)
    }

    pub fn date_time(self) -> DateTime<Utc> {
        self.0
    }
}

impl FromStr for ReadMarker {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .map(Self)
    }
}

impl fmt::Display for ReadMarker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.to_rfc3339_opts(SecondsFormat::Millis, true).fmt(f)
    }
}

pub fn latest_triggers_unread(messages: &[Message]) -> Option<DateTime<Utc>> {
    messages
        .iter()
        .rev()
        .find(|message| message.triggers_unread())
        .map(|message| message.server_time)
}

pub fn latest_can_reference(messages: &[Message]) -> Option<MessageReferences> {
    messages
        .iter()
        .rev()
        .find(|message| message.can_reference())
        .map(Message::references)
}

pub async fn load(kind: Kind) -> Result<Metadata, Error> {
    let path = path(&kind).await?;

    if let Ok(bytes) = fs::read(path).await {
        Ok(serde_json::from_slice(&bytes).unwrap_or_default())
    } else {
        Ok(Metadata::default())
    }
}

pub async fn save(
    kind: &Kind,
    messages: &[Message],
    read_marker: Option<ReadMarker>,
) -> Result<(), Error> {
    let bytes = serde_json::to_vec(&Metadata {
        read_marker,
        last_triggers_unread: latest_triggers_unread(messages),
        chathistory_references: latest_can_reference(messages),
    })?;

    let path = path(kind).await?;

    fs::write(path, &bytes).await?;

    Ok(())
}

pub async fn update(
    kind: &Kind,
    read_marker: &ReadMarker,
) -> Result<(), Error> {
    let metadata = load(kind.clone()).await?;

    if metadata.read_marker.is_some_and(|metadata_read_marker| {
        metadata_read_marker >= *read_marker
    }) {
        return Ok(());
    }

    let bytes = serde_json::to_vec(&Metadata {
        read_marker: Some(*read_marker),
        last_triggers_unread: metadata.last_triggers_unread,
        chathistory_references: metadata.chathistory_references,
    })?;

    let path = path(kind).await?;

    fs::write(path, &bytes).await?;

    Ok(())
}

async fn path(kind: &Kind) -> Result<PathBuf, Error> {
    let dir = dir_path().await?;

    let name = match kind {
        Kind::Server(server) => format!("{server}-metadata"),
        Kind::Channel(server, channel) => {
            format!("{server}channel{}-metadata", channel.as_normalized_str())
        }
        Kind::Query(server, query) => {
            format!("{server}nickname{}-metadata", query.as_normalized_str())
        }
        Kind::Logs => "logs-metadata".to_string(),
        Kind::Highlights => "highlights-metadata".to_string(),
    };

    let hashed_name = seahash::hash(name.as_bytes());

    Ok(dir.join(format!("{hashed_name}.json")))
}
