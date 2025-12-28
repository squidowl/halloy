use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct ContextMenu {
    pub padding: Padding,
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Padding {
    pub entry: [u16; 2],
}

impl Default for Padding {
    fn default() -> Self {
        Self { entry: [5, 5] }
    }
}
