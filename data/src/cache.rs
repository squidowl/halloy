mod trim;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use derive_more::AsRef;
use reqwest::header::{self, HeaderMap, HeaderValue};
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

#[derive(Debug)]
pub struct FetchResponse<T> {
    pub value: T,
    pub response_headers: HeaderMap,
}

pub trait CachedAsset {
    fn assets(&self) -> Vec<Asset<'_>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheState<T> {
    Ok(T),
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry<T> {
    state: CacheState<T>,
    freshness: HttpFreshness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HttpFreshness {
    max_age_secs: u64,
    age_at_store_secs: u64,
    stored_at_unix_secs: i64,
}

const FRESHNESS_FACTOR: u64 = 10;

impl HttpFreshness {
    fn is_fresh_at(&self, now: DateTime<Utc>) -> bool {
        let elapsed_secs = now
            .timestamp()
            .saturating_sub(self.stored_at_unix_secs)
            .max(0) as u64;

        let current_age = self.age_at_store_secs.saturating_add(elapsed_secs);
        current_age < self.max_age_secs
    }

    fn from_headers(headers: &HeaderMap) -> Option<Self> {
        Self::from_headers_at(headers, Utc::now())
    }

    fn from_headers_at(
        headers: &HeaderMap,
        now: DateTime<Utc>,
    ) -> Option<Self> {
        let cache_control = headers
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok());

        if cache_control.is_some_and(contains_no_store_or_no_cache) {
            return None;
        }

        let age = parse_age(headers);

        if let Some(max_age_secs) = parse_max_age(cache_control) {
            return Some(Self {
                max_age_secs,
                age_at_store_secs: age,
                stored_at_unix_secs: now.timestamp(),
            });
        }

        let date = headers
            .get(header::DATE)
            .and_then(parse_http_date)
            .unwrap_or(now);

        let resident_age =
            now.signed_duration_since(date).num_seconds().max(0) as u64;

        let age_at_store_secs = age.max(resident_age);

        let max_age_secs = if let Some(expires) =
            headers.get(header::EXPIRES).and_then(parse_http_date)
        {
            expires.signed_duration_since(date).num_seconds().max(0) as u64
        } else {
            let last_modified = headers
                .get(header::LAST_MODIFIED)
                .and_then(parse_http_date)?;

            let since_last_modified = date
                .signed_duration_since(last_modified)
                .num_seconds()
                .max(0) as u64;

            since_last_modified / FRESHNESS_FACTOR
        };

        Some(Self {
            max_age_secs,
            age_at_store_secs,
            stored_at_unix_secs: now.timestamp(),
        })
    }
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
        let entry: CacheEntry<T> = serde_json::from_slice(&bytes).ok()?;

        if let CacheState::Ok(ref asset) = entry.state {
            let assets = asset.assets();
            if verify_assets(&assets).await.is_none() {
                remove_assets(&assets).await;
                return None;
            }
            if !entry.freshness.is_fresh_at(Utc::now()) {
                remove_assets(&assets).await;
                return None;
            }
        }

        Some(entry.state)
    }

    pub async fn save<T: Serialize + Clone>(
        &self,
        url: &Url,
        state: CacheState<T>,
        headers: &HeaderMap,
    ) {
        let Some(freshness) = HttpFreshness::from_headers(headers) else {
            return;
        };

        if !freshness.is_fresh_at(Utc::now()) {
            return;
        }

        let path = self.state_path(url);

        if let Some(parent) = path.parent().filter(|p| !p.exists()) {
            let _ = fs::create_dir_all(parent).await;
        }

        let entry = CacheEntry { state, freshness };

        let Ok(bytes) = serde_json::to_vec(&entry) else {
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

fn contains_no_store_or_no_cache(cache_control: &str) -> bool {
    cache_control.split(',').any(|directive| {
        matches!(
            directive.trim().to_ascii_lowercase().as_str(),
            "no-store" | "no-cache"
        )
    })
}

fn parse_max_age(cache_control: Option<&str>) -> Option<u64> {
    let cache_control = cache_control?;

    cache_control.split(',').find_map(|directive| {
        let (key, value) = directive.trim().split_once('=')?;

        if key.trim().eq_ignore_ascii_case("max-age") {
            value.trim().trim_matches('"').parse().ok()
        } else {
            None
        }
    })
}

fn parse_age(headers: &HeaderMap) -> u64 {
    headers
        .get(header::AGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|age| age.trim().parse().ok())
        .unwrap_or_default()
}

fn parse_http_date(value: &HeaderValue) -> Option<DateTime<Utc>> {
    let date = httpdate::parse_http_date(value.to_str().ok()?).ok()?;
    Some(DateTime::from(date))
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};
    use reqwest::header::{
        AGE, CACHE_CONTROL, DATE, EXPIRES, HeaderMap, HeaderValue,
        LAST_MODIFIED,
    };

    use super::HttpFreshness;
    use crate::cache::parse_max_age;

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 10, 12, 0, 0).unwrap()
    }

    fn fmt_date(dt: DateTime<Utc>) -> HeaderValue {
        HeaderValue::from_str(&httpdate::fmt_http_date(dt.into())).unwrap()
    }

    #[test]
    fn parse_max_age_directive() {
        assert_eq!(parse_max_age(Some("max-age=60")), Some(60));
        assert_eq!(parse_max_age(Some("public, max-age=120")), Some(120));
        assert_eq!(parse_max_age(Some("max-age=\"90\"")), Some(90));
        assert_eq!(parse_max_age(Some("no-cache")), None);
        assert_eq!(parse_max_age(None), None);
    }

    #[test]
    fn max_age_is_preferred_over_expires() {
        let now = fixed_now();

        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=60"));
        headers.insert(EXPIRES, fmt_date(now + chrono::Duration::seconds(518)));
        headers.insert(DATE, fmt_date(now));

        let freshness = HttpFreshness::from_headers_at(&headers, now)
            .expect("freshness should be parsed");

        assert_eq!(freshness.max_age_secs, 60);
        assert_eq!(freshness.age_at_store_secs, 0);
    }

    #[test]
    fn age_header_affects_freshness() {
        let now = fixed_now();

        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=100"));
        headers.insert(AGE, HeaderValue::from_static("30"));

        let freshness = HttpFreshness::from_headers_at(&headers, now)
            .expect("freshness should be parsed");

        assert_eq!(freshness.max_age_secs, 100);
        assert_eq!(freshness.age_at_store_secs, 30);
    }

    #[test]
    fn stale_response_when_age_exceeds_max_age() {
        let now = fixed_now();

        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=60"));
        headers.insert(AGE, HeaderValue::from_static("90"));

        let freshness = HttpFreshness::from_headers_at(&headers, now)
            .expect("freshness should be parsed");

        assert!(!freshness.is_fresh_at(now));
    }

    #[test]
    fn last_modified_sets_max_age() {
        let now = fixed_now();
        let delta: u64 = 120;

        let mut headers = HeaderMap::new();
        headers.insert(DATE, fmt_date(now));
        headers.insert(
            LAST_MODIFIED,
            fmt_date(now - chrono::Duration::seconds(delta as i64)),
        );

        let freshness = HttpFreshness::from_headers_at(&headers, now)
            .expect("freshness should be parsed");

        assert_eq!(freshness.max_age_secs, delta / super::FRESHNESS_FACTOR);
    }

    #[test]
    fn expires_sets_max_age() {
        let now = fixed_now();

        let mut headers = HeaderMap::new();
        headers.insert(DATE, fmt_date(now));
        headers.insert(EXPIRES, fmt_date(now + chrono::Duration::seconds(60)));

        let freshness = HttpFreshness::from_headers_at(&headers, now)
            .expect("freshness should be parsed");

        assert_eq!(freshness.max_age_secs, 60);
    }

    #[test]
    fn no_store_is_not_cacheable() {
        let now = fixed_now();

        let mut headers = HeaderMap::new();
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        headers.insert(
            LAST_MODIFIED,
            fmt_date(now - chrono::Duration::seconds(100)),
        );

        assert!(HttpFreshness::from_headers_at(&headers, now).is_none());
    }

    #[test]
    fn freshness_over_time() {
        let now = fixed_now();
        let freshness = HttpFreshness {
            max_age_secs: 60,
            age_at_store_secs: 0,
            stored_at_unix_secs: now.timestamp(),
        };

        assert!(freshness.is_fresh_at(now + chrono::Duration::seconds(30)));
        assert!(!freshness.is_fresh_at(now + chrono::Duration::seconds(61)));
    }
}
