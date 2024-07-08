use serde::Deserialize;

use crate::audio::Sound;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Notification {
    #[serde(rename = "enabled", default)]
    pub show_toast: bool,
    pub sound: Option<Sound>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Notifications {
    #[serde(default)]
    pub connected: Notification,
    #[serde(default)]
    pub disconnected: Notification,
    #[serde(default)]
    pub reconnected: Notification,
    #[serde(default)]
    pub highlight: Notification,
    #[serde(default)]
    pub file_transfer_request: Notification,
}
