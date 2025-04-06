use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Away {
    #[serde(default)]
    pub appearance: Appearance,
}

impl Away {
    pub fn appearance(&self, is_user_away: bool) -> Option<Appearance> {
        is_user_away.then_some(self.appearance)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Appearance {
    Dimmed(Option<f32>),
    Solid,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance::Dimmed(None)
    }
}

impl<'de> Deserialize<'de> for Appearance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum AppearanceRepr {
            String(String),
            Struct(DimmedStruct),
        }

        #[derive(Deserialize)]
        struct DimmedStruct {
            dimmed: Option<f32>,
        }

        let repr = AppearanceRepr::deserialize(deserializer)?;
        match repr {
            AppearanceRepr::String(s) => match s.as_str() {
                "dimmed" => Ok(Appearance::Dimmed(None)),
                "solid" => Ok(Appearance::Solid),
                _ => Err(serde::de::Error::custom(format!(
                    "unknown appearance: {s}",
                ))),
            },
            AppearanceRepr::Struct(s) => Ok(Appearance::Dimmed(s.dimmed)),
        }
    }
}
