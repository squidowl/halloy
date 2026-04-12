use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Display {
    pub direction_arrows: DirectionArrows,
    pub decode_urls: bool,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            direction_arrows: DirectionArrows::default(),
            decode_urls: true,
        }
    }
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
