use serde::{Deserialize, Serialize};

use crate::buffer;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub buffer: buffer::Settings,
}
