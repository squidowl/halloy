use chrono::TimeDelta;
use serde::{Deserialize, Deserializer};

use crate::buffer::{Alignment, Brackets, Color};
use crate::config::buffer::{Away, NicknameClickAction};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Nickname {
    pub away: Away,
    pub offline: Offline,
    pub color: Color,
    pub brackets: Brackets,
    pub alignment: Alignment,
    pub show_access_levels: bool,
    pub click: NicknameClickAction,
    pub shown_status: ShownStatus,
    pub truncate: Option<u16>,
    pub hide_consecutive: HideConsecutive,
}

impl Default for Nickname {
    fn default() -> Self {
        Self {
            away: Away::default(),
            offline: Offline::default(),
            color: Color::default(),
            brackets: Brackets::default(),
            alignment: Alignment::default(),
            show_access_levels: true,
            click: NicknameClickAction::default(),
            shown_status: ShownStatus::default(),
            truncate: None,
            hide_consecutive: HideConsecutive::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Offline {
    #[default]
    Solid,
    None,
}

impl Offline {
    pub fn is_offline(&self, is_user_offline: bool) -> bool {
        is_user_offline && matches!(self, Offline::Solid)
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShownStatus {
    #[default]
    Current,
    Historical,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum HideConsecutive {
    #[default]
    Disabled,
    Enabled(Option<TimeDelta>),
}

impl<'de> Deserialize<'de> for HideConsecutive {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        pub enum Inner {
            Boolean(bool),
            Smart { smart: i64 },
        }

        match Inner::deserialize(deserializer)? {
            Inner::Boolean(enabled) => {
                if enabled {
                    Ok(HideConsecutive::Enabled(None))
                } else {
                    Ok(HideConsecutive::Disabled)
                }
            }
            Inner::Smart { smart } => {
                Ok(HideConsecutive::Enabled(TimeDelta::try_seconds(smart)))
            }
        }
    }
}
