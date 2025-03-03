use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::pane::Pane;
use crate::{buffer, compression, environment};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub pane: Pane,
    #[serde(default)]
    pub popout_panes: Vec<Pane>,
    #[serde(default)]
    pub buffer_settings: BufferSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BufferSettings(HashMap<String, buffer::Settings>);

impl BufferSettings {
    pub fn get(&self, buffer: &buffer::Buffer) -> Option<&buffer::Settings> {
        self.0.get(&buffer.key())
    }

    pub fn insert(&mut self, buffer: buffer::Buffer, settings: &buffer::Settings) {
        self.0.insert(buffer.key(), settings.clone());
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BufferAction {
    #[default]
    NewPane,
    ReplacePane,
    NewWindow,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BufferFocusedAction {
    #[default]
    ClosePane,
}

impl Dashboard {
    pub fn load() -> Result<Self, Error> {
        let path = path()?;

        let bytes = std::fs::read(path)?;

        Ok(compression::decompress(&bytes)?)
    }

    pub async fn save(self) -> Result<(), Error> {
        let path = path()?;

        let bytes = compression::compress(&self)?;

        tokio::fs::write(path, &bytes).await?;

        Ok(())
    }
}

fn path() -> Result<PathBuf, Error> {
    let parent = environment::data_dir();

    if !parent.exists() {
        std::fs::create_dir_all(&parent)?;
    }

    Ok(parent.join("dashboard.json.gz"))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Compression(#[from] compression::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}
