use chrono::Locale;
use serde::Deserialize;

use super::Brackets;
use crate::buffer::deserialize_locale;
use crate::config::buffer::HideConsecutive;
use crate::serde::{
    deserialize_strftime_date, deserialize_strftime_date_maybe,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Timestamp {
    #[serde(deserialize_with = "deserialize_strftime_date")]
    pub format: String,
    pub brackets: Brackets,
    #[serde(deserialize_with = "deserialize_strftime_date")]
    pub context_menu_format: String,
    #[serde(deserialize_with = "deserialize_strftime_date_maybe")]
    pub copy_format: Option<String>,
    #[serde(deserialize_with = "deserialize_locale")]
    pub locale: Locale,
    pub hide_consecutive: HideConsecutive,
}

impl Default for Timestamp {
    fn default() -> Self {
        Self {
            format: "%R".to_string(),
            brackets: Brackets::default(),
            context_menu_format: "%x".to_string(),
            copy_format: None,
            locale: Locale::default(),
            hide_consecutive: HideConsecutive::default(),
        }
    }
}
