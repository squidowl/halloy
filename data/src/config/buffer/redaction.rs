use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Redaction {
    pub display: Display,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Display {
    #[default]
    None,
    Redacted,
    Dimmed,
}

impl Display {
    pub fn is_visible(self) -> bool {
        matches!(self, Self::Redacted | Self::Dimmed)
    }

    pub fn is_redacted(self) -> bool {
        matches!(self, Self::Redacted)
    }
}
