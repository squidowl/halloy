use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const MIN_WIDTH: f32 = 426.0;
    pub const MIN_HEIGHT: f32 = 240.0;

    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width: width.max(Self::MIN_WIDTH),
            height: height.max(Self::MIN_HEIGHT),
        }
    }
}

impl Default for Size {
    fn default() -> Self {
        Self {
            width: 1024.0,
            height: 768.0,
        }
    }
}

impl From<Size> for iced_core::Size {
    fn from(size: Size) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}
