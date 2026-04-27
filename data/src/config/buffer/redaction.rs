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
    Dimmed,
}

impl Display {
    pub fn is_dimmed(self) -> bool {
        matches!(self, Self::Dimmed)
    }
}
