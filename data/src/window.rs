use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use iced_core::{Point, Size};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::environment;

pub const MIN_SIZE: Size = Size::new(426.0, 240.0);

pub mod position;
pub mod size;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Window {
    #[serde(with = "serde_position")]
    pub position: Option<Point>,
    #[serde(with = "serde_size")]
    pub size: Size,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            position: None,
            size: Size {
                width: 1024.0,
                height: 768.0,
            },
        }
    }
}

impl Window {
    pub async fn load() -> Result<Window, Error> {
        let path = path()?;
        let bytes = fs::read(path).await?;
        let Window { position, size } = serde_json::from_slice(&bytes)?;

        let size = size.max(MIN_SIZE);
        let position = position
            .filter(|pos| pos.y.is_sign_positive() && pos.x.is_sign_positive())
            .filter(|pos| is_position_valid(*pos));

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

/// Check if a window position is valid (within visible screen bounds)
fn is_position_valid(position: Point) -> bool {
    // Get all available displays
    let displays = match display_info::DisplayInfo::all() {
        Ok(displays) => displays,
        Err(_) => return true, // If we can't get display info, assume it's valid
    };

    if displays.is_empty() {
        return true; // No displays detected, assume valid
    }

    // Check if the window position is within any display bounds
    // We only check the position, not the full window size, to handle different monitor sizes
    for display in displays {
        if position.x >= display.x as f32
            && position.y >= display.y as f32
            && position.x < (display.x + display.width as i32) as f32
            && position.y < (display.y + display.height as i32) as f32
        {
            return true;
        }
    }

    // Window position is not within any display bounds
    false
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Serde(Arc<serde_json::Error>),
    #[error(transparent)]
    Io(Arc<io::Error>),
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Serde(Arc::new(error))
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
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
