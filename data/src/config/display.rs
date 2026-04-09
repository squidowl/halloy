use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Display {
    pub direction_arrows: DirectionArrows,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DirectionArrows {
    pub left: String,
    pub right: String,
}

impl Default for DirectionArrows {
    fn default() -> Self {
        Self {
            left: String::from("←"),
            right: String::from("→"),
        }
    }
}
