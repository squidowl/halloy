use std::collections::HashMap;
use std::ops::Deref;

use serde::{Deserialize, Deserializer, de};

use crate::buffer::SkinTone;
use crate::emoji;

#[derive(Debug, Clone, Default)]
pub struct Aliases(pub HashMap<String, String>);

impl Deref for Aliases {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Aliases {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = HashMap::<String, String>::deserialize(deserializer)?;

        let mut resolved = HashMap::with_capacity(raw.len());
        for (alias, value) in raw {
            let shortcode =
                resolve_shortcode(&value).map_err(de::Error::custom)?;
            resolved.insert(alias, shortcode);
        }

        Ok(Aliases(resolved))
    }
}

fn resolve_shortcode(value: &str) -> Result<String, String> {
    if let Some(e) = emojis::get(value) {
        let code = e
            .shortcode()
            .map_or_else(|| emoji::synthetic_shortcode(e), str::to_string);
        return Ok(format!(":{code}:"));
    }

    let stripped = value.trim_matches(':');
    if emoji::get_by_shortcode(stripped).is_none() {
        return Err(format!(
            "invalid emoji alias value `{value}`: expected an emoji character or a :shortcode:"
        ));
    }

    Ok(format!(":{stripped}:"))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Emojis {
    pub show_picker: bool,
    pub skin_tone: SkinTone,
    pub auto_replace: bool,
    pub characters_to_trigger_picker: usize,
    pub aliases: Aliases,
}

impl Default for Emojis {
    fn default() -> Self {
        Self {
            show_picker: true,
            skin_tone: SkinTone::default(),
            auto_replace: true,
            characters_to_trigger_picker: 2,
            aliases: Aliases::default(),
        }
    }
}
