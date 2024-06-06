use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{compression, environment};

pub use self::size::Size;
pub use self::position::Position;

pub mod size;
pub mod position;


#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Window {
    pub position: Option<Position>,
    pub size: Size,
}

impl Window {
    pub fn update(self, event: Event) -> Self {
        match event {
            Event::Moved(position) => Self { position: Some(position), ..self },
            Event::Resized(size) => Self { size, ..self },
        }
    }

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

    Ok(parent.join("window.json.gz"))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Compression(#[from] compression::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Moved(Position),
    Resized(Size),
}