use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Broadcast {
    pub disconnected: bool,
    pub reconnected: bool,
}

impl Default for Broadcast {
    fn default() -> Self {
        Self {
            disconnected: true,
            reconnected: true,
        }
    }
}
