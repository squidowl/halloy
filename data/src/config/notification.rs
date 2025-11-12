use std::collections::HashMap;

use serde::Deserialize;

use crate::audio::Sound;
use crate::config::inclusivities::{Inclusivities, is_user_included};
use crate::isupport;
use crate::server::Server;
use crate::target::Channel;
use crate::user::User;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Notification {
    pub show_toast: bool,
    pub show_content: bool,
    pub sound: Option<String>,
    pub delay: Option<u32>,
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
}

impl Default for Notification {
    fn default() -> Self {
        Self {
            show_toast: false,
            show_content: false,
            sound: None,
            delay: Some(500),
            exclude: None,
            include: None,
        }
    }
}

impl Notification {
    pub fn should_notify(
        &self,
        user: &User,
        channel: Option<&Channel>,
        server: &Server,
        casemapping: isupport::CaseMap,
    ) -> bool {
        is_user_included(
            self.include.as_ref(),
            self.exclude.as_ref(),
            user,
            channel,
            server,
            casemapping,
        )
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Notifications {
    pub connected: Notification,
    pub disconnected: Notification,
    pub reconnected: Notification,
    pub direct_message: Notification,
    pub highlight: Notification,
    pub file_transfer_request: Notification,
    pub monitored_online: Notification,
    pub monitored_offline: Notification,
    #[serde(rename = "channel")]
    pub channels: HashMap<String, Notification>,
}

impl Notifications {
    pub fn load_sounds<'a>(
        &self,
        highlight_matches_sounds: impl Iterator<Item = &'a str>,
    ) -> HashMap<String, Sound> {
        let mut sounds = HashMap::new();

        // Helper function to load a sound and add it to the map
        let mut load_and_insert = |name: &str| {
            if !sounds.contains_key(name) {
                match Sound::load(name) {
                    Ok(sound) => {
                        sounds.insert(name.to_string(), sound);
                    }
                    Err(e) => {
                        log::warn!("Failed to load sound '{name}': {e}");
                    }
                }
            }
        };

        // Load sounds from each notification
        if let Some(sound_name) = self.connected.sound.as_deref() {
            load_and_insert(sound_name);
        }
        if let Some(sound_name) = self.disconnected.sound.as_deref() {
            load_and_insert(sound_name);
        }
        if let Some(sound_name) = self.reconnected.sound.as_deref() {
            load_and_insert(sound_name);
        }
        if let Some(sound_name) = self.direct_message.sound.as_deref() {
            load_and_insert(sound_name);
        }
        if let Some(sound_name) = self.highlight.sound.as_deref() {
            load_and_insert(sound_name);
        }
        if let Some(sound_name) = self.file_transfer_request.sound.as_deref() {
            load_and_insert(sound_name);
        }
        if let Some(sound_name) = self.monitored_online.sound.as_deref() {
            load_and_insert(sound_name);
        }
        if let Some(sound_name) = self.monitored_offline.sound.as_deref() {
            load_and_insert(sound_name);
        }
        for notification in self.channels.values() {
            if let Some(sound_name) = notification.sound.as_deref() {
                load_and_insert(sound_name);
            }
        }
        for sound_name in highlight_matches_sounds {
            load_and_insert(sound_name);
        }

        sounds
    }
}
