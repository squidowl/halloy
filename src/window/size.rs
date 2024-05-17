#[derive(Debug, Copy, Clone, Default)]
pub struct Size {
    width: f32,
    height: f32
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width,  height}
    }
}