use serde::Deserialize;
use std::collections::HashMap;

use crate::buffer::SkinTone;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Emojis {
    pub show_picker: bool,
    pub skin_tone: SkinTone,
    pub auto_replace: bool,
    pub characters_to_trigger_picker: usize,
    pub aliases: HashMap<String, String>,
}

impl Default for Emojis {
    fn default() -> Self {
        Self {
            show_picker: true,
            skin_tone: SkinTone::default(),
            auto_replace: true,
            characters_to_trigger_picker: 2,
            aliases: HashMap::new(),
        }
    }
}
