use serde::{Deserialize, Deserializer};

use crate::buffer::{Alignment, Brackets, Color};
use crate::config::buffer::{
    AccessLevelFormat, Alpha, Dimmed, HideConsecutive,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Nickname {
    pub away: Alpha,
    pub offline: Offline,
    pub color: Color,
    pub brackets: Brackets,
    pub alignment: Alignment,
    pub show_access_levels: AccessLevelFormat,
    pub show_bot_icon: bool,
    pub shown_status: ShownStatus,
    pub truncate: Option<u16>,
    pub hide_consecutive: HideConsecutive,
}

impl Default for Nickname {
    fn default() -> Self {
        Self {
            away: Alpha::default(),
            offline: Offline::default(),
            color: Color::default(),
            brackets: Brackets::default(),
            alignment: Alignment::default(),
            show_access_levels: AccessLevelFormat::default(),
            show_bot_icon: true,
            shown_status: ShownStatus::default(),
            truncate: None,
            hide_consecutive: HideConsecutive::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Offline {
    Enabled(OfflineStyle),
    None,
}

#[derive(Debug, Clone, Copy)]
pub struct OfflineStyle {
    pub color: OfflineColor,
    pub alpha: Alpha,
}

fn default_offline_alpha() -> Alpha {
    Alpha::default()
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OfflineColor {
    #[default]
    Theme,
    Nickname,
}

impl Default for Offline {
    fn default() -> Self {
        Offline::Enabled(OfflineStyle {
            color: OfflineColor::Theme,
            alpha: default_offline_alpha(),
        })
    }
}

impl Offline {
    pub fn style(&self, is_user_offline: bool) -> Option<OfflineStyle> {
        match (is_user_offline, self) {
            (true, Offline::Enabled(style)) => Some(*style),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for Offline {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum AppearanceRepr {
            String(String),
            DimmedShorthand(DimmedShorthand),
            Struct(OfflineStyleRepr),
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct DimmedShorthand {
            dimmed: Option<f32>,
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct OfflineStyleRepr {
            #[serde(default)]
            color: OfflineColor,
            #[serde(default = "default_offline_alpha")]
            alpha: Alpha,
        }

        match AppearanceRepr::deserialize(deserializer)? {
            AppearanceRepr::String(s) => match s.as_str() {
                "solid" => Ok(Offline::Enabled(OfflineStyle {
                    color: OfflineColor::Theme,
                    alpha: Alpha::None,
                })),
                "dimmed" => Ok(Offline::Enabled(OfflineStyle {
                    color: OfflineColor::Nickname,
                    alpha: Alpha::Dimmed(Dimmed::default()),
                })),
                "none" => Ok(Offline::None),
                _ => Err(serde::de::Error::custom(format!(
                    "unknown appearance: {s}"
                ))),
            },
            AppearanceRepr::DimmedShorthand(s) => {
                Ok(Offline::Enabled(OfflineStyle {
                    color: OfflineColor::Nickname,
                    alpha: Alpha::Dimmed(Dimmed {
                        enabled: true,
                        alpha: s.dimmed,
                    }),
                }))
            }
            AppearanceRepr::Struct(s) => Ok(Offline::Enabled(OfflineStyle {
                color: s.color,
                alpha: s.alpha,
            })),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShownStatus {
    #[default]
    Current,
    Historical,
}
