use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Default, Deserialize, Serialize)]
pub struct Position {
    x: f32,
    y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<Position> for iced_core::Point {
    fn from(position: Position) -> Self {
        Self {
            x: position.x,
            y: position.y,
        }
    }
}

impl From<Position> for iced_core::window::Position {
    fn from(position: Position) -> Self {
        Self::Specific(position.into())
    }
}
