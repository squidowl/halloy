use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::cache::HexDigest;

pub type Format = image::ImageFormat;
pub type Error = image::ImageError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    #[serde(with = "serde_format")]
    pub format: Format,
    pub url: Url,
    pub digest: HexDigest,
    pub path: PathBuf,
}

impl Image {
    pub fn new(
        format: Format,
        url: Url,
        digest: HexDigest,
        path: PathBuf,
    ) -> Self {
        Self {
            format,
            url,
            digest,
            path,
        }
    }
}

pub fn format(bytes: &[u8]) -> Option<Format> {
    image::guess_format(bytes).ok()
}

mod serde_format {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::Format;

    pub fn serialize<S: Serializer>(
        format: &Format,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        format.to_mime_type().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Format, D::Error> {
        let s = String::deserialize(deserializer)?;

        Format::from_mime_type(s)
            .ok_or(serde::de::Error::custom("invalid mime type"))
    }
}
