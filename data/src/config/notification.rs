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

impl Notifications {
    pub fn load_sound_data(&mut self) {
        self.connected.sound.as_mut().map(|s| s.load_data());
        self.disconnected.sound.as_mut().map(|s| s.load_data());
        self.reconnected.sound.as_mut().map(|s| s.load_data());
        self.highlight.sound.as_mut().map(|sound| sound.load_data());
        self.file_transfer_request
            .sound
            .as_mut()
            .map(|s| s.load_data());
    }
}
