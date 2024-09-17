use chrono::{format::SecondsFormat, DateTime, Utc};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::history::{dir_path, Error, Kind};
use crate::{server, Message};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Metadata {
    pub read_marker: Option<DateTime<Utc>>,
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

pub fn after_read_marker(message: &Message, read_marker: &Option<DateTime<Utc>>) -> bool {
    read_marker.is_none()
        || read_marker.is_some_and(|read_marker| message.server_time > read_marker)
}

pub fn read_marker_to_string(read_marker: &Option<DateTime<Utc>>) -> String {
    if let Some(read_marker) = read_marker {
        read_marker.to_rfc3339_opts(SecondsFormat::Millis, true)
    } else {
        "*".to_string()
    }
}
