mod trim;

use std::path::{Path, PathBuf};

use chrono::Utc;
use derive_more::AsRef;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::fs;
use url::Url;

pub use trim::TrimConfig;

/// SHA256 digest of cache content.
#[derive(Debug, Clone, Serialize, Deserialize, AsRef)]
pub struct Digest(String);

impl Digest {
    pub fn new(data: &[u8]) -> Self {
        Self(hex::encode(data))
    }
}

pub trait CachedAsset {
    fn paths(&self) -> Vec<&Path>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheState<T> {
    Ok(T),
    Error,
}

pub struct FileCache {
    root: PathBuf,
    trim: TrimConfig,
}

impl FileCache {
    pub fn new(root: PathBuf, trim: TrimConfig) -> Self {
        Self { root, trim }
    }

    pub async fn load<T>(&self, url: &Url) -> Option<CacheState<T>>
    where
        T: CachedAsset + DeserializeOwned,
    {
        let path = self.state_path(url);

        let bytes = fs::read(&path).await.ok()?;
        let state: CacheState<T> = serde_json::from_slice(&bytes).ok()?;

        if let CacheState::Ok(ref asset) = state {
            let any_missing = asset.paths().iter().any(|p| !p.exists());
            if any_missing {
                // If any of the asset's files are missing, treat the cache as invalid.
                return None;
            }
        }

        Some(state)
    }

    pub async fn save<T: Serialize>(&self, url: &Url, state: &CacheState<T>) {
        let path = self.state_path(url);

        if let Some(parent) = path.parent().filter(|p| !p.exists()) {
            let _ = fs::create_dir_all(parent).await;
        }

        let Ok(bytes) = serde_json::to_vec(state) else {
            return;
        };
        let _ = fs::write(&path, &bytes).await;
    }

    pub fn account_blob(&self, size: u64, blob_path: PathBuf) {
        self.trim.maybe_trim(size, blob_path);
    }

    pub fn state_path(&self, url: &Url) -> PathBuf {
        let hash =
            hex::encode(seahash::hash(url.as_str().as_bytes()).to_be_bytes());

        self.root
            .join("state")
            .join(&hash[..2])
            .join(&hash[2..4])
            .join(&hash[4..6])
            .join(format!("{hash}.json"))
    }

    pub fn blob_path(&self, digest: &Digest, ext: &str) -> PathBuf {
        let hash = digest.as_ref();

        blob_dir_from_root(&self.root)
            .join(&hash[..2])
            .join(&hash[2..4])
            .join(&hash[4..6])
            .join(format!("{hash}.{ext}"))
    }

    pub fn download_path(&self, url: &Url) -> PathBuf {
        let hash = seahash::hash(url.as_str().as_bytes());
        let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();

        self.root
            .join("downloads")
            .join(format!("{hash}-{nanos}.part"))
    }
}

pub fn blob_dir_from_root(root: &Path) -> PathBuf {
    root.join("blobs")
}
