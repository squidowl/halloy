use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Away {
    #[serde(default)]
    pub appearance: Appearance,
}

impl Away {
    pub fn should_dim_nickname(&self, is_user_away: bool) -> bool {
        is_user_away && matches!(self.appearance, Appearance::Dimmed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Appearance {
    #[default]
    Dimmed,
    Solid,
}
