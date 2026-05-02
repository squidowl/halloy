use serde::Deserialize;

use crate::config::preview::Exclude;
use crate::metadata;
use crate::metadata::Key::{self, *};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Metadata {
    /// List of keys to subscribe to, in order of preference
    pub preferred_keys: Vec<metadata::Key>,
    /// Avatar-specific loading policy.
    pub avatar: Avatar,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            preferred_keys: vec![
                DisplayName,
                Avatar,
                Pronouns,
                Homepage,
                Color,
                Status,
            ],
            avatar: Avatar::default(),
        }
    }
}

impl Metadata {
    pub fn preferred_key_strs(&self) -> impl Iterator<Item = &'static str> {
        self.preferred_keys.iter().copied().map(Key::to_str)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Avatar {
    pub enabled: bool,
    pub exclude: Exclude,
}

impl Default for Avatar {
    fn default() -> Self {
        Self {
            enabled: true,
            exclude: Exclude::default(),
        }
    }
}

impl Avatar {
    pub fn is_enabled(&self, url: &str) -> bool {
        self.enabled && !self.exclude.is_excluded(url)
    }
}
