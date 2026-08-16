use fancy_regex::{Regex, RegexBuilder};
use iced::Color as IcedColor;
use serde::{Deserialize, Deserializer};

use crate::appearance::theme::hex_to_color;
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
    #[serde(rename = "color_override")]
    pub color_overrides: Vec<ColorOverride>,
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
            color_overrides: vec![],
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

impl Nickname {
    pub fn color_override(&self, nickname: &str) -> Option<IcedColor> {
        self.color_overrides
            .iter()
            .find(|rule| rule.is_match(nickname))
            .map(|rule| rule.color)
    }
}

#[derive(Debug, Clone)]
pub struct ColorOverride {
    matcher: Matcher,
    color: IcedColor,
}

impl ColorOverride {
    fn is_match(&self, nickname: &str) -> bool {
        match &self.matcher {
            Matcher::Nicknames {
                nicknames,
                case_insensitive,
            } => nicknames.iter().any(|configured| {
                if *case_insensitive {
                    configured.eq_ignore_ascii_case(nickname)
                } else {
                    configured == nickname
                }
            }),
            Matcher::Regex(regex) => {
                regex.is_match(nickname).is_ok_and(|is_match| is_match)
            }
        }
    }
}

#[derive(Debug, Clone)]
enum Matcher {
    Nicknames {
        nicknames: Vec<String>,
        case_insensitive: bool,
    },
    Regex(Regex),
}

impl<'de> Deserialize<'de> for ColorOverride {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Nicknames {
                nicknames: Vec<String>,
                #[serde(default = "default_true")]
                case_insensitive: bool,
                color: String,
            },
            Regex {
                regex: String,
                color: String,
            },
        }

        let (matcher, color) = match Repr::deserialize(deserializer)? {
            Repr::Nicknames {
                nicknames,
                case_insensitive,
                color,
            } => {
                if nicknames.is_empty() {
                    return Err(serde::de::Error::custom(
                        "nickname color override must contain at least one nickname",
                    ));
                }

                (
                    Matcher::Nicknames {
                        nicknames,
                        case_insensitive,
                    },
                    color,
                )
            }
            Repr::Regex { regex, color } => {
                let compiled =
                    RegexBuilder::new(&regex).build().map_err(|err| {
                        serde::de::Error::custom(format!(
                            "invalid nickname color regex '{regex}': {err}"
                        ))
                    })?;

                (Matcher::Regex(compiled), color)
            }
        };

        let color = hex_to_color(&color).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "invalid nickname override color: {color}"
            ))
        })?;

        Ok(Self { matcher, color })
    }
}

fn default_true() -> bool {
    true
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
