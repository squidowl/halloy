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
    pub adapt_metadata_colors: AdaptMetadataColors,
}

impl Default for Display {
    fn default() -> Self {
        Self {
            direction_arrows: DirectionArrows::default(),
            decode_urls: true,
            truncation_character: '…',
            nickname: vec![nickname::Metadata::DisplayName],
            nicklist_nickname: vec![nickname::Metadata::DisplayName],
            adapt_metadata_colors: AdaptMetadataColors::default(),
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

#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptMetadataColors {
    #[default]
    All,
    Illegible,
    None,
}
