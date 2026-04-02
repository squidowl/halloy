use serde::Deserialize;

use crate::serde::deserialize_u8_positive_integer_maybe;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Typing {
    pub share: bool,
    pub show: bool,
    pub style: Style,
    #[serde(deserialize_with = "deserialize_u8_positive_integer_maybe")]
    pub font_size: Option<u8>,
    pub animation: Animation,
}

impl Default for Typing {
    fn default() -> Self {
        Self {
            share: false,
            show: true,
            style: Default::default(),
            font_size: None,
            animation: Animation::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct Animation {
    pub enabled: bool,
    #[serde(deserialize_with = "deserialize_u8_positive_integer_maybe")]
    pub size: Option<u8>,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            enabled: true,
            size: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Style {
    Padded,
    #[default]
    Popped,
}
