use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Icon {
    pub enabled: bool,
}

impl Default for Icon {
    fn default() -> Self {
        Self { enabled: true }
    }
}
