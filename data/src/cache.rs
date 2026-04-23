mod trim;

use std::path::{Path, PathBuf};

use chrono::Utc;
use derive_more::AsRef;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;
pub(crate) use trim::TrimConfig;
use url::Url;

/// SHA256 digest of cache content.
#[derive(Debug, Clone, Serialize, Deserialize, AsRef)]
pub struct HexDigest(String);

impl HexDigest {
    pub fn new(data: &[u8]) -> Self {
        Self(hex::encode(data))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Asset<'a>(pub &'a Path, pub &'a HexDigest);

pub trait CachedAsset {
    fn assets(&self) -> Vec<Asset<'_>>;
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
    pub fn new(
        root: PathBuf,
        max_size: Option<u64>,
        trim_interval: u64,
    ) -> Self {
        let trim =
            TrimConfig::new(blob_dir_from_root(&root), max_size, trim_interval);

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
            let assets = asset.assets();
            if verify_assets(&assets).await.is_none() {
                remove_assets(&assets).await;
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

    pub fn blob_path(&self, digest: &HexDigest, ext: &str) -> PathBuf {
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

async fn hash_file(file: fs::File) -> Option<HexDigest> {
    let mut stream = ReaderStream::new(file);
    let mut hasher = Sha256::new();

    while let Some(item) = stream.next().await {
        let chunk = item.ok()?;
        hasher.update(&chunk);
    }

    Some(HexDigest::new(&hasher.finalize()))
}

async fn verify_assets(assets: &[Asset<'_>]) -> Option<()> {
    for Asset(path, digest) in assets {
        // Check if the file actually exists
        let file = fs::File::open(path).await.ok()?;

        // Check if the file content matches the expected digest
        let actual_digest = hash_file(file).await?;
        if actual_digest.as_ref() != digest.as_ref() {
            return None;
        }
    }

    Some(())
}

async fn remove_assets(assets: &[Asset<'_>]) {
    for Asset(path, _) in assets {
        let _ = fs::remove_file(path).await;
    }
}
