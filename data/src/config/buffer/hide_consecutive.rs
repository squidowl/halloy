use chrono::TimeDelta;
use serde::{Deserialize, Deserializer};

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

#[cfg(test)]
mod tests {
    use super::{HideConsecutive, HideConsecutiveEnabled};

    #[test]
    fn hide_consecutive_deserializes_boolean_true() {
        let config: HideConsecutive =
            toml::from_str("enabled = true").expect("valid hide_consecutive");

        assert!(matches!(
            config.enabled,
            HideConsecutiveEnabled::Enabled(None)
        ));
        assert!(!config.show_after_previews);
    }

    #[test]
    fn hide_consecutive_deserializes_smart() {
        let config: HideConsecutive =
            toml::from_str("enabled = { smart = 42 }")
                .expect("valid hide_consecutive smart");

        assert!(matches!(
            config.enabled,
            HideConsecutiveEnabled::Enabled(Some(duration))
                if duration.num_seconds() == 42
        ));
        assert!(!config.show_after_previews);
    }

    #[test]
    fn hide_consecutive_deserializes_direct_boolean() {
        #[derive(serde::Deserialize)]
        struct Root {
            hide_consecutive: HideConsecutive,
        }

        let config: Root = toml::from_str("hide_consecutive = true")
            .expect("valid direct boolean value");

        assert!(matches!(
            config.hide_consecutive.enabled,
            HideConsecutiveEnabled::Enabled(None)
        ));
        assert!(!config.hide_consecutive.show_after_previews);
    }

    #[test]
    fn hide_consecutive_deserializes_show_after_previews() {
        let config: HideConsecutive =
            toml::from_str("enabled = true\nshow_after_previews = true")
                .expect("valid show_after_previews value");

        assert!(matches!(
            config.enabled,
            HideConsecutiveEnabled::Enabled(None)
        ));
        assert!(config.show_after_previews);
    }
}
