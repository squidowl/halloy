use serde::Deserialize;

use crate::serde::deserialize_u8_positive_integer_maybe;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Typing {
    pub share: bool,
    pub show: bool,
    #[serde(deserialize_with = "deserialize_u8_positive_integer_maybe")]
    pub font_size: Option<u8>,
}

impl Default for Typing {
    fn default() -> Self {
        Self {
            share: false,
            show: true,
            font_size: None,
        }
    }
}
