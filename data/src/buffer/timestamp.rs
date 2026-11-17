use icu_locale::Locale;
use serde::Deserialize;

use super::Brackets;
use crate::config::buffer::HideConsecutive;
use crate::serde::deserialize_locale;
use crate::time::system_locale;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Timestamp {
    pub format: String,
    pub brackets: Brackets,
    pub context_menu_format: Option<String>,
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
            context_menu_format: None,
            copy_format: None,
            locale: system_locale(),
            hide_consecutive: HideConsecutive::default(),
        }
    }
}
