#[derive(Debug, Copy, Clone, Default)]
pub struct Position {
    x: f32,
    y: f32
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x,  y}
    }
}