use serde::{Deserialize, Serialize};
use url::Url;

use super::Image;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub url: Url,
    pub canonical_url: Url,
    pub image: Image,
    pub title: String,
    pub description: Option<String>,
}
