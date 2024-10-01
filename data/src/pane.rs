use serde::{Deserialize, Serialize};

use crate::{buffer, Buffer};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Pane {
    Split {
        axis: Axis,
        ratio: f32,
        a: Box<Pane>,
        b: Box<Pane>,
    },
    Buffer {
        buffer: Buffer,
        settings: buffer::Settings,
    },
    Empty,
    FileTransfers,
    Logs,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Axis {
    Horizontal,
    Vertical,
}
