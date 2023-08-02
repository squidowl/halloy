use serde::Deserialize;

#[cfg(target_os = "macos")]
const DEFAULT_SOUND: &str = "Submarine";
#[cfg(all(unix, not(target_os = "macos")))]
const DEFAULT_SOUND: &str = "message-new-instant";
#[cfg(target_os = "windows")]
const DEFAULT_SOUND: &str = "Mail";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Notification {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_sound")]
    sound: String,
    #[serde(default)]
    mute: bool,
}

impl Notification {
    pub fn sound(&self) -> Option<&str> {
        (!self.mute).then_some(&self.sound)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Notifications {
    #[serde(default)]
    pub connected: Notification,
    #[serde(default)]
    pub disconnected: Notification,
    #[serde(default)]
    pub reconnected: Notification,
}

fn default_sound() -> String {
    DEFAULT_SOUND.to_string()
}
