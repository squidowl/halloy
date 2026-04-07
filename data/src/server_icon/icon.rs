use std::path::PathBuf;

use derive_more::derive::AsRef;
use serde::{Deserialize, Serialize};
use url::Url;

use super::cache;

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
pub struct Icon {
    pub url: Url,
    pub digest: Digest,
    pub path: PathBuf,
}

impl Icon {
    pub fn new(url: Url, digest: Digest) -> Self {
        let path = cache::image_path(&digest);

        Self { url, digest, path }
    }
}
