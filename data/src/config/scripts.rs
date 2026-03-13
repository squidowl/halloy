use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Scripts {
    pub autorun: Vec<String>,
}
