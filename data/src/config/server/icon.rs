use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Icon {
    pub enabled: bool,
    pub override_url: Option<String>,
}

impl Default for Icon {
    fn default() -> Self {
        Self {
            enabled: true,
            override_url: None,
        }
    }
}
