use std::path::PathBuf;

use derive_more::derive::AsRef;
use serde::{Deserialize, Serialize};
use url::Url;

use super::cache;

pub type Format = image::ImageFormat;
pub type Error = image::ImageError;

/// SHA256 digest of image
#[derive(Debug, Clone, Serialize, Deserialize, AsRef)]
pub struct Digest(String);

impl Digest {
    pub fn new(data: &[u8]) -> Self {
        Self(hex::encode(data))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    #[serde(with = "serde_format")]
    pub format: Format,
    pub url: Url,
    pub digest: Digest,
    pub path: PathBuf,
}

impl Image {
    pub fn new(format: Format, url: Url, digest: Digest) -> Self {
        let path = cache::image_path(&format, &digest);

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
    use super::Format;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(format: &Format, serializer: S) -> Result<S::Ok, S::Error> {
        format.to_mime_type().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Format, D::Error> {
        let s = String::deserialize(deserializer)?;

        Format::from_mime_type(s).ok_or(serde::de::Error::custom("invalid mime type"))
    }
}
