use serde::Deserialize;

pub mod nickname;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Display {
    pub direction_arrows: DirectionArrows,
    pub decode_urls: bool,
    pub truncation_character: char,
    pub nickname: Vec<nickname::Metadata>,
    pub nicklist_nickname: Vec<nickname::Metadata>,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            direction_arrows: DirectionArrows::default(),
            decode_urls: true,
            truncation_character: '…',
            nickname: vec![nickname::Metadata::DisplayName],
            nicklist_nickname: vec![nickname::Metadata::DisplayName],
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
