use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};

use crate::buffer::{self, Buffer};
use crate::pane::Pane;
use crate::{compression, environment};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Dashboard {
    pub pane: Pane,
    pub popout_panes: Vec<Pane>,
    pub buffer_settings: BufferSettings,
    pub focus_buffer: Option<Buffer>,
    pub sidebar: Sidebar,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BufferSettings {
    settings: HashMap<String, buffer::Settings>,
    pub show_muted: bool,
}

impl<'de> Deserialize<'de> for BufferSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        pub enum Format {
            BufferSettings {
                #[serde(default)]
                settings: HashMap<String, buffer::Settings>,
                #[serde(default)]
                show_muted: bool,
            },
            Legacy(HashMap<String, buffer::Settings>),
        }

        match Format::deserialize(deserializer)? {
            Format::BufferSettings {
                settings,
                show_muted,
            } => Ok(BufferSettings {
                settings,
                show_muted,
            }),
            Format::Legacy(settings) => Ok(BufferSettings {
                settings,
                ..BufferSettings::default()
            }),
        }
    }
}

impl BufferSettings {
    pub fn get(&self, buffer: &buffer::Buffer) -> Option<&buffer::Settings> {
        self.settings.get(&buffer.key())
    }

    pub fn entry(
        &mut self,
        buffer: &buffer::Buffer,
        maybe_default: Option<buffer::Settings>,
    ) -> &mut buffer::Settings {
        self.settings
            .entry(buffer.key())
            .or_insert_with(|| maybe_default.unwrap_or_default())
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sidebar {
    Hidden,
    #[default]
    Visible,
}

impl Sidebar {
    pub fn is_hidden(self) -> bool {
        matches!(self, Self::Hidden)
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
    pub fn exists() -> Result<bool, Error> {
        let path = path()?;

        Ok(std::fs::exists(path)?)
    }

    pub async fn load() -> Result<Self, Error> {
        let path = path()?;
        let file = tokio::fs::File::open(&path).await?;
        let reader = tokio::io::BufReader::new(file);

        Ok(compression::decompress_and_deserialize(reader).await?)
    }

    pub async fn save(self) -> Result<(), Error> {
        let path = path()?;
        let mut file = tokio::fs::File::create(&path).await?;

        compression::serialize_and_compress(&self, &mut file).await?;

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
