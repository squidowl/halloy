use std::cmp::Reverse;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::SystemTime;

use chrono::Utc;
use log;
use serde::{Deserialize, Serialize};
use tokio::fs;
use url::Url;
use walkdir::WalkDir;

use super::{Preview, image};
use crate::{config, environment};

const IMAGE_CACHE_TRIMMED_FRACTION_NUMERATOR: u64 = 3;
const IMAGE_CACHE_TRIMMED_FRACTION_DENOMINATOR: u64 = 4;
const IMAGE_CACHE_TRIM_INTERVAL: u64 = 32;
static IMAGE_CACHE_SAVE_COUNTER: AtomicU64 = AtomicU64::new(0);
static IMAGE_CACHE_SIZE_ESTIMATE: AtomicU64 = AtomicU64::new(0);
static IMAGE_CACHE_FIRST_SAVE_SEEN: AtomicBool = AtomicBool::new(false);
static IMAGE_CACHE_TRIM_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Ok(Preview),
    Error,
}

pub async fn load(
    url: &Url,
    client: Arc<reqwest::Client>,
    config: &config::Preview,
) -> Option<State> {
    let path = state_path(url);

    if !path.exists() {
        return None;
    }

    let state: State =
        serde_json::from_slice(&fs::read(&path).await.ok()?).ok()?;

    // Ensure the actual image is cached
    match &state {
        State::Ok(Preview::Card(card)) => {
            if !card.image.path.exists() {
                super::fetch(card.image.url.clone(), client, config)
                    .await
                    .ok()?;
            }
        }
        State::Ok(Preview::Image(image)) => {
            if !image.path.exists() {
                super::fetch(image.url.clone(), client, config).await.ok()?;
            }
        }
        State::Error => {}
    }

    Some(state)
}

pub async fn save(url: &Url, state: State) {
    let path = state_path(url);

    if let Some(parent) = path.parent().filter(|p| !p.exists()) {
        let _ = fs::create_dir_all(parent).await;
    }

    let Ok(bytes) = serde_json::to_vec(&state) else {
        return;
    };

    let _ = fs::write(path, &bytes).await;
}

pub(super) fn maybe_trim_image_cache(
    image_size: u64,
    max_image_cache_size: u64,
    protected_path: PathBuf,
) {
    if max_image_cache_size == 0 {
        return;
    }

    let written_size = IMAGE_CACHE_SIZE_ESTIMATE
        .fetch_add(image_size, Ordering::Relaxed)
        + image_size;
    let saves = IMAGE_CACHE_SAVE_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let first_save_in_session =
        !IMAGE_CACHE_FIRST_SAVE_SEEN.swap(true, Ordering::Relaxed);

    if !should_trim_on_save(
        written_size,
        max_image_cache_size,
        saves,
        first_save_in_session,
    ) {
        return;
    }

    if IMAGE_CACHE_TRIM_IN_PROGRESS
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    tokio::spawn(async move {
        trim_image_cache_once(max_image_cache_size, protected_path).await;
        IMAGE_CACHE_TRIM_IN_PROGRESS.store(false, Ordering::Release);
    });
}

async fn trim_image_cache_once(
    max_image_cache_size: u64,
    protected_path: PathBuf,
) {
    let cache_root = environment::cache_dir().join("previews").join("images");

    let mut files =
        tokio::task::spawn_blocking(move || collect_cache_files(&cache_root))
            .await
            .unwrap_or_default();

    let cached_files =
        find_cached_files(&mut files, max_image_cache_size, &protected_path);

    if cached_files.is_empty() {
        return;
    }

    let removed_files = cached_files.len();
    let removed_bytes = cached_files.iter().map(|file| file.size).sum::<u64>();

    for file in &cached_files {
        let _ = fs::remove_file(&file.path).await;
    }

    log::debug!(
        "trimmed preview image cache: removed {removed_files} files ({removed_mb:.2} MB) to enforce max {max_mb:.2} MB",
        removed_mb = bytes_to_mb(removed_bytes),
        max_mb = bytes_to_mb(max_image_cache_size),
    );
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1_000_000.0
}

fn state_path(url: &Url) -> PathBuf {
    let hash =
        hex::encode(seahash::hash(url.as_str().as_bytes()).to_be_bytes());

    environment::cache_dir()
        .join("previews")
        .join("state")
        .join(&hash[..2])
        .join(&hash[2..4])
        .join(&hash[4..6])
        .join(format!("{hash}.json"))
}

pub(super) fn download_path(url: &Url) -> PathBuf {
    let hash = seahash::hash(url.as_str().as_bytes());
    // Unique download path so if 2 identical URLs are downloading
    // at the same time, they don't clobber eachother
    let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();

    environment::cache_dir()
        .join("previews")
        .join("downloads")
        .join(format!("{hash}-{nanos}.part"))
}

pub(super) fn image_path(
    format: &image::Format,
    digest: &image::Digest,
) -> PathBuf {
    environment::cache_dir()
        .join("previews")
        .join("images")
        .join(&digest.as_ref()[..2])
        .join(&digest.as_ref()[2..4])
        .join(&digest.as_ref()[4..6])
        .join(format!(
            "{}.{}",
            digest.as_ref(),
            format.extensions_str()[0]
        ))
}

#[derive(Debug, Clone)]
struct CacheFile {
    path: PathBuf,
    size: u64,
    system_time: SystemTime,
}

fn collect_cache_files(root: &Path) -> Vec<CacheFile> {
    if !root.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        if let Ok(metadata) = entry.metadata() {
            let system_time = metadata
                .accessed()
                .or(metadata.modified())
                .or(metadata.created())
                .unwrap_or(SystemTime::UNIX_EPOCH);

            files.push(CacheFile {
                path: entry.path().to_path_buf(),
                size: metadata.len(),
                system_time,
            });
        }
    }

    files
}

fn find_cached_files(
    files: &mut Vec<CacheFile>,
    max_size: u64,
    path: &Path,
) -> Vec<CacheFile> {
    let mut total_size = files.iter().map(|file| file.size).sum::<u64>();

    if total_size <= max_size {
        return Vec::new();
    }

    files.sort_by_key(|file| Reverse(file.system_time));

    let mut cached_files = Vec::new();

    let trimmed_size = (IMAGE_CACHE_TRIMMED_FRACTION_NUMERATOR * max_size)
        .div_ceil(IMAGE_CACHE_TRIMMED_FRACTION_DENOMINATOR);

    while total_size > trimmed_size {
        let Some(file) = files.pop() else {
            break;
        };

        if file.path == path {
            continue;
        }

        total_size = total_size.saturating_sub(file.size);
        cached_files.push(file);
    }

    IMAGE_CACHE_SIZE_ESTIMATE.store(total_size, Ordering::Release);

    cached_files
}

fn should_trim_on_save(
    written_size: u64,
    max_size: u64,
    saves: u64,
    first_save_in_session: bool,
) -> bool {
    first_save_in_session
        || (written_size > max_size)
        || saves.is_multiple_of(IMAGE_CACHE_TRIM_INTERVAL)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    use super::{CacheFile, find_cached_files};

    #[test]
    fn find_cached_files_removes_oldest_until_under_limit() {
        let base = SystemTime::UNIX_EPOCH;
        let mut files = vec![
            CacheFile {
                path: PathBuf::from("a"),
                size: 5,
                system_time: base + Duration::from_secs(1),
            },
            CacheFile {
                path: PathBuf::from("b"),
                size: 5,
                system_time: base + Duration::from_secs(2),
            },
            CacheFile {
                path: PathBuf::from("c"),
                size: 5,
                system_time: base + Duration::from_secs(3),
            },
        ];

        let cached_files = find_cached_files(&mut files, 10, Path::new("none"));

        assert_eq!(cached_files.len(), 2);
        assert_eq!(cached_files[0].path, PathBuf::from("a"));
    }

    #[test]
    fn find_cached_files_returns_empty_when_under_limit() {
        let mut files = vec![CacheFile {
            path: PathBuf::from("a"),
            size: 5,
            system_time: SystemTime::UNIX_EPOCH,
        }];

        let cached_files = find_cached_files(&mut files, 5, Path::new("none"));

        assert!(cached_files.is_empty());
    }
}
