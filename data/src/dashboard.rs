use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::pane::Pane;
use crate::{compression, environment};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub pane: Pane,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub enum DefaultAction {
    #[default]
    NewPane,
    ReplacePane,
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
