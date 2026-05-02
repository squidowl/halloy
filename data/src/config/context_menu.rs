use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ContextMenu {
    pub padding: Padding,
    pub show_user_metadata: bool,
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self {
            padding: Padding::default(),
            show_user_metadata: true,
        }
    }
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
