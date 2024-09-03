use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::environment;

pub use self::position::Position;
pub use self::size::Size;

pub mod position;
pub mod size;

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Window {
    pub position: Option<Position>,
    pub size: Size,
    pub focused: bool,
}

impl Window {
    pub fn update(&mut self, event: Event) -> Self {
        match event {
            Event::Moved(position) => {
                self.position = Some(position);
            }
            Event::Resized(size) => {
                self.size = size;
            }
            Event::Focused => {
                self.focused = true;
            }
            Event::Unfocused => {
                self.focused = false;
            }
        }
        *self
    }

    pub fn load() -> Result<Self, Error> {
        let path = path()?;

        let bytes = std::fs::read(path)?;

        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn save(self) -> Result<(), Error> {
        let path = path()?;

        let bytes = serde_json::to_vec(&self)?;
        tokio::fs::write(path, &bytes).await?;

        Ok(())
    }
}

fn path() -> Result<PathBuf, Error> {
    let parent = environment::data_dir();

    if !parent.exists() {
        std::fs::create_dir_all(&parent)?;
    }

    Ok(parent.join("window.json"))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Moved(Position),
    Resized(Size),
    Focused,
    Unfocused,
}
