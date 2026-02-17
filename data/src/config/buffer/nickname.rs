use chrono::TimeDelta;
use serde::{Deserialize, Deserializer};

use crate::buffer::{Alignment, Brackets, Color};
use crate::config::buffer::{AccessLevelFormat, Away, NicknameClickAction};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Nickname {
    pub away: Away,
    pub offline: Offline,
    pub color: Color,
    pub brackets: Brackets,
    pub alignment: Alignment,
    pub show_access_levels: AccessLevelFormat,
    pub click: NicknameClickAction,
    pub shown_status: ShownStatus,
    pub truncate: Option<u16>,
    pub hide_consecutive: HideConsecutive,
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
pub struct HideConsecutive {
    pub enabled: HideConsecutiveEnabled,
    pub show_after_previews: bool,
}

impl<'de> Deserialize<'de> for HideConsecutive {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        enum Inner {
            Struct {
                enabled: HideConsecutiveEnabled,
                #[serde(default)]
                show_after_previews: bool,
            },
            Boolean(bool),
            Smart {
                smart: i64,
            },
        }

        match Inner::deserialize(deserializer)? {
            Inner::Struct {
                enabled,
                show_after_previews,
            } => Ok(HideConsecutive {
                enabled,
                show_after_previews,
            }),
            Inner::Boolean(enabled) => Ok(HideConsecutive {
                enabled: if enabled {
                    HideConsecutiveEnabled::Enabled(None)
                } else {
                    HideConsecutiveEnabled::Disabled
                },
                show_after_previews: false,
            }),
            Inner::Smart { smart } => Ok(HideConsecutive {
                enabled: HideConsecutiveEnabled::Enabled(
                    TimeDelta::try_seconds(smart),
                ),
                show_after_previews: false,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum HideConsecutiveEnabled {
    #[default]
    Disabled,
    Enabled(Option<TimeDelta>),
}

impl<'de> Deserialize<'de> for HideConsecutiveEnabled {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        enum Inner {
            Boolean(bool),
            Smart { smart: i64 },
        }

        match Inner::deserialize(deserializer)? {
            Inner::Boolean(enabled) => {
                if enabled {
                    Ok(HideConsecutiveEnabled::Enabled(None))
                } else {
                    Ok(HideConsecutiveEnabled::Disabled)
                }
            }
            Inner::Smart { smart } => Ok(HideConsecutiveEnabled::Enabled(
                TimeDelta::try_seconds(smart),
            )),
        }
    }
}
