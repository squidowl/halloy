use std::cmp::Reverse;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::fs;
use url::Url;
use walkdir::WalkDir;

use super::icon::Icon;
use crate::environment;
use crate::server_icon::icon;

const ICON_CACHE_TRIMMED_FRACTION_NUMERATOR: u64 = 3;
const ICON_CACHE_TRIMMED_FRACTION_DENOMINATOR: u64 = 4;
const ICON_CACHE_MAX_SIZE_BYTES: u64 = 50_000_000;
const ICON_CACHE_TRIM_INTERVAL: u64 = 32;
static ICON_CACHE_SAVE_COUNTER: AtomicU64 = AtomicU64::new(0);
static ICON_CACHE_SIZE_ESTIMATE: AtomicU64 = AtomicU64::new(0);
static ICON_CACHE_FIRST_SAVE_SEEN: AtomicBool = AtomicBool::new(false);
static ICON_CACHE_TRIM_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Ok(Icon),
    Error,
}

pub async fn load(url: &Url, client: Arc<reqwest::Client>) -> Option<State> {
    let path = state_path(url);

    if !path.exists() {
        return None;
    }

    let state: State =
        serde_json::from_slice(&fs::read(&path).await.ok()?).ok()?;

    // Ensure the actual image is cached
    match &state {
        State::Ok(icon) => {
            if !icon.path.exists() {
                super::fetch(icon.url.clone(), client).await.ok()?;
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

pub(super) fn image_path(
    format: &image::ImageFormat,
    digest: &icon::Digest,
) -> PathBuf {
    environment::cache_dir()
        .join("server_icons")
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

pub(super) fn maybe_trim_icon_cache(image_size: u64, protected_path: PathBuf) {
    let written_size = ICON_CACHE_SIZE_ESTIMATE
        .fetch_add(image_size, Ordering::Relaxed)
        + image_size;
    let saves = ICON_CACHE_SAVE_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let first_save_in_session =
        !ICON_CACHE_FIRST_SAVE_SEEN.swap(true, Ordering::Relaxed);

    if !should_trim_on_save(
        written_size,
        ICON_CACHE_MAX_SIZE_BYTES,
        saves,
        ICON_CACHE_TRIM_INTERVAL,
        first_save_in_session,
    ) {
        return;
    }

    if ICON_CACHE_TRIM_IN_PROGRESS
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    tokio::spawn(async move {
        trim_icon_cache_once(ICON_CACHE_MAX_SIZE_BYTES, protected_path).await;
        ICON_CACHE_TRIM_IN_PROGRESS.store(false, Ordering::Release);
    });
}

fn state_path(url: &Url) -> PathBuf {
    let hash =
        hex::encode(seahash::hash(url.as_str().as_bytes()).to_be_bytes());

    environment::cache_dir()
        .join("server_icons")
        .join("state")
        .join(&hash[..2])
        .join(&hash[2..4])
        .join(&hash[4..6])
        .join(format!("{hash}.json"))
}

async fn trim_icon_cache_once(max_cache_size: u64, protected_path: PathBuf) {
    let cache_root =
        environment::cache_dir().join("server_icons").join("images");

    let mut files =
        tokio::task::spawn_blocking(move || collect_cache_files(&cache_root))
            .await
            .unwrap_or_default();

    let cached_files =
        find_cached_files(&mut files, max_cache_size, &protected_path);

    if cached_files.is_empty() {
        return;
    }

    let removed_files = cached_files.len();
    let removed_bytes = cached_files.iter().map(|file| file.size).sum::<u64>();

    for file in &cached_files {
        let _ = fs::remove_file(&file.path).await;
    }

    log::debug!(
        "trimmed server icon cache: removed {removed_files} files ({removed_mb:.2} MB) to enforce max {max_mb:.2} MB",
        removed_mb = bytes_to_mb(removed_bytes),
        max_mb = bytes_to_mb(max_cache_size),
    );
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1_000_000.0
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

    for entry in WalkDir::new(root)
        .follow_root_links(false)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
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

    let trimmed_size = (ICON_CACHE_TRIMMED_FRACTION_NUMERATOR * max_size)
        .div_ceil(ICON_CACHE_TRIMMED_FRACTION_DENOMINATOR);

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

    ICON_CACHE_SIZE_ESTIMATE.store(total_size, Ordering::Release);

    cached_files
}

fn should_trim_on_save(
    written_size: u64,
    max_size: u64,
    saves: u64,
    trim_interval: u64,
    first_save_in_session: bool,
) -> bool {
    first_save_in_session
        || (trim_interval != 0
            && ((written_size > max_size)
                || saves.is_multiple_of(trim_interval)))
}
