use serde::Deserialize;

pub use crate::config::animation::Animation;
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
            style: Style::default(),
            font_size: None,
            animation: Animation::default(),
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
