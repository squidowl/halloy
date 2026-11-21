use serde::{Deserialize, Deserializer};

use crate::serde::deserialize_u8_positive_integer_maybe;

#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Animation {
    #[serde(deserialize_with = "deserialize_typing_animation_enabled")]
    pub enabled: AnimationEnabled,
    #[serde(deserialize_with = "deserialize_u8_positive_integer_maybe")]
    pub size: Option<u8>,
}

impl Animation {
    pub fn is_disabled(&self) -> bool {
        matches!(self.enabled, AnimationEnabled::Never)
    }

    pub fn is_enabled(&self) -> bool {
        !self.is_disabled()
    }

    pub fn is_enabled_always(&self) -> bool {
        matches!(self.enabled, AnimationEnabled::Always)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationEnabled {
    Always,
    #[default]
    Focused,
    Never,
}

pub fn deserialize_typing_animation_enabled<'de, D>(
    deserializer: D,
) -> Result<AnimationEnabled, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Inner {
        String(AnimationEnabled),
        Boolean(bool),
    }

    match Inner::deserialize(deserializer)? {
        Inner::String(string) => Ok(string),
        Inner::Boolean(boolean) => match boolean {
            true => Ok(AnimationEnabled::Focused),
            false => Ok(AnimationEnabled::Never),
        },
    }
}
