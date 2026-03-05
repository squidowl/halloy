use serde::Deserialize;

#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Window {
    pub initial_height: Option<u32>,
    pub initial_width: Option<u32>,
}
