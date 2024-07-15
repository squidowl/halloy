use serde::Deserialize;

use crate::audio::{self, Sound};

pub type Loaded = Notification<Sound>;

#[derive(Debug, Clone, Deserialize)]
pub struct Notification<T = String> {
    #[serde(rename = "enabled", default)]
    pub show_toast: bool,
    pub sound: Option<T>,
}

impl<T> Default for Notification<T> {
    fn default() -> Self {
        Self {
            show_toast: false,
            sound: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Notifications<T = String> {
    #[serde(default)]
    pub connected: Notification<T>,
    #[serde(default)]
    pub disconnected: Notification<T>,
    #[serde(default)]
    pub reconnected: Notification<T>,
    #[serde(default)]
    pub highlight: Notification<T>,
    #[serde(default)]
    pub file_transfer_request: Notification<T>,
}

impl<T> Default for Notifications<T> {
    fn default() -> Self {
        Self {
            connected: Notification::default(),
            disconnected: Notification::default(),
            reconnected: Notification::default(),
            highlight: Notification::default(),
            file_transfer_request: Notification::default(),
        }
    }
}

impl Notifications {
    pub fn load_sounds(&self) -> Result<Notifications<Sound>, audio::LoadError> {
        let load = |notification: &Notification<String>| -> Result<_, audio::LoadError> {
            Ok(Notification {
                show_toast: notification.show_toast,
                sound: notification.sound.as_deref().map(Sound::load).transpose()?,
            })
        };

        Ok(Notifications {
            connected: load(&self.connected)?,
            disconnected: load(&self.disconnected)?,
            reconnected: load(&self.reconnected)?,
            highlight: load(&self.highlight)?,
            file_transfer_request: load(&self.file_transfer_request)?,
        })
    }
}
