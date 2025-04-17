use std::io;
use std::path::PathBuf;

use iced_core::{Point, Size};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::environment;

pub const MIN_SIZE: Size = Size::new(426.0, 240.0);

pub mod position;
pub mod size;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Window {
    #[serde(default, with = "serde_position")]
    pub position: Option<Point>,
    #[serde(default = "default_size", with = "serde_size")]
    pub size: Size,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            position: None,
            size: default_size(),
        }
    }
}

pub fn default_size() -> Size {
    Size {
        width: 1024.0,
        height: 768.0,
    }
}

impl Window {
    pub async fn load() -> Result<Window, Error> {
        let path = path()?;
        let bytes = fs::read(path).await?;
        let Window { position, size } = serde_json::from_slice(&bytes)?;

        let size = size.max(MIN_SIZE);
        let position = position
            .filter(|pos| pos.y.is_sign_positive() && pos.x.is_sign_positive());

        Ok(Window { position, size })
    }

    pub async fn save(self) -> Result<(), Error> {
        let path = path()?;

        let bytes = serde_json::to_vec(&self)?;
        fs::write(path, &bytes).await?;

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

mod serde_position {
    use serde::{Deserializer, Serializer};

    use super::*;

    #[derive(Deserialize, Serialize)]
    struct SerdePosition {
        x: f32,
        y: f32,
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<Point>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let maybe = Option::<SerdePosition>::deserialize(deserializer)?;

        Ok(maybe.map(|SerdePosition { x, y }| Point { x, y }))
    }

    pub fn serialize<S: Serializer>(
        position: &Option<Point>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        position
            .map(|p| SerdePosition { x: p.x, y: p.y })
            .serialize(serializer)
    }
}

mod serde_size {
    use serde::{Deserializer, Serializer};

    use super::*;

    #[derive(Deserialize, Serialize)]
    struct SerdeSize {
        width: f32,
        height: f32,
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Size, D::Error>
    where
        D: Deserializer<'de>,
    {
        let SerdeSize { width, height } = SerdeSize::deserialize(deserializer)?;

        Ok(Size { width, height })
    }

    pub fn serialize<S: Serializer>(
        size: &Size,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        SerdeSize {
            width: size.width,
            height: size.height,
        }
        .serialize(serializer)
    }
}
