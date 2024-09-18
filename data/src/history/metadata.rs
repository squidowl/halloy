use chrono::{format::SecondsFormat, DateTime, Utc};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::history::{dir_path, Error, Kind};
use crate::{message, server, Message};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Metadata {
    pub read_marker: Option<ReadMarker>,
}

impl Metadata {
    pub fn merge(self, other: Self) -> Self {
        Self {
            read_marker: self
                .read_marker
                .zip(other.read_marker)
                .map(|(a, b)| a.max(b))
                .or(self.read_marker)
                .or(other.read_marker),
        }
    }

    pub fn update_read_marker(&mut self, read_marker: ReadMarker) {
        self.read_marker = Some(
            self.read_marker
                .map_or(read_marker, |marker| marker.max(read_marker)),
        );
    }

    pub fn updated(self, messages: &[Message]) -> Self {
        Self {
            read_marker: ReadMarker::latest(messages).or(self.read_marker),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize, Serialize)]
pub struct ReadMarker(DateTime<Utc>);

impl ReadMarker {
    pub fn latest(messages: &[Message]) -> Option<Self> {
        messages
            .iter()
            .rev()
            .find(|message| !matches!(message.target.source(), message::Source::Internal(_)))
            .map(|message| message.server_time)
            .map(Self)
    }

    pub fn date_time(&self) -> DateTime<Utc> {
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

pub async fn load(server: server::Server, kind: Kind) -> Result<Metadata, Error> {
    let path = path(&server, &kind).await?;

    if let Ok(bytes) = fs::read(path).await {
        Ok(serde_json::from_slice(&bytes).unwrap_or_default())
    } else {
        Ok(Metadata::default())
    }
}

pub async fn save(server: &server::Server, kind: &Kind, metadata: &Metadata) -> Result<(), Error> {
    let bytes = serde_json::to_vec(metadata)?;

    let path = path(server, kind).await?;

    fs::write(path, &bytes).await?;

    Ok(())
}

async fn path(server: &server::Server, kind: &Kind) -> Result<PathBuf, Error> {
    let dir = dir_path().await?;

    let name = match kind {
        Kind::Server => format!("{server}-metadata"),
        Kind::Channel(channel) => format!("{server}channel{channel}-metadata"),
        Kind::Query(nick) => format!("{server}nickname{}-metadata", nick),
    };

    let hashed_name = seahash::hash(name.as_bytes());

    Ok(dir.join(format!("{hashed_name}.json")))
}
