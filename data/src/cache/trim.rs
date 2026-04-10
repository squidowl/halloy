use std::cmp::Reverse;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::SystemTime;

use walkdir::WalkDir;

const TRIM_TARGET_NUMERATOR: u64 = 3;
const TRIM_TARGET_DENOMINATOR: u64 = 4;

#[derive(Clone)]
pub struct TrimConfig {
    blob_root: PathBuf,
    max_size: Option<u64>,
    interval: u64,
    save_counter: Arc<AtomicU64>,
    size_estimate: Arc<AtomicU64>,
    first_save_seen: Arc<AtomicBool>,
    trim_in_progress: Arc<AtomicBool>,
}

impl TrimConfig {
    pub fn new(
        blob_root: PathBuf,
        max_size: Option<u64>,
        interval: u64,
    ) -> Self {
        Self {
            blob_root,
            max_size,
            interval,
            save_counter: Arc::new(AtomicU64::new(0)),
            size_estimate: Arc::new(AtomicU64::new(0)),
            first_save_seen: Arc::new(AtomicBool::new(false)),
            trim_in_progress: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(super) fn maybe_trim(&self, size: u64, protected_path: PathBuf) {
        let Some(max_size) = self.max_size else {
            return;
        };

        let written_size =
            self.size_estimate.fetch_add(size, Ordering::Relaxed) + size;
        let saves = self.save_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let first_save = !self.first_save_seen.swap(true, Ordering::Relaxed);

        if !should_trim(
            written_size,
            max_size,
            saves,
            self.interval,
            first_save,
        ) {
            return;
        }

        if self
            .trim_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        let blob_root = self.blob_root.clone();
        let size_estimate = self.size_estimate.clone();
        let trim_in_progress = self.trim_in_progress.clone();

        tokio::spawn(async move {
            trim_once(blob_root, max_size, protected_path, size_estimate).await;
            trim_in_progress.store(false, Ordering::Release);
        });
    }
}

fn should_trim(
    written_size: u64,
    max_size: u64,
    saves: u64,
    interval: u64,
    first_save: bool,
) -> bool {
    first_save
        || (interval != 0
            && ((written_size > max_size) || saves.is_multiple_of(interval)))
}

async fn trim_once(
    blob_root: PathBuf,
    max_size: u64,
    protected_path: PathBuf,
    size_estimate: Arc<AtomicU64>,
) {
    let mut files =
        tokio::task::spawn_blocking(move || collect_files(&blob_root))
            .await
            .unwrap_or_default();

    let to_remove = files_to_remove(&mut files, max_size, &protected_path);

    if to_remove.is_empty() {
        return;
    }

    let removed_count = to_remove.len();
    let removed_bytes = to_remove.iter().map(|f| f.size).sum::<u64>();

    for file in &to_remove {
        let _ = tokio::fs::remove_file(&file.path).await;
    }

    let remaining: u64 = files
        .iter()
        .filter(|f| !to_remove.iter().any(|r| r.path == f.path))
        .map(|f| f.size)
        .sum();
    size_estimate.store(remaining, Ordering::Release);

    log::debug!(
        "file_cache: trimmed {removed_count} file(s) ({removed_mb:.2} MB removed, \
         max {max_mb:.2} MB)",
        removed_mb = bytes_to_mb(removed_bytes),
        max_mb = bytes_to_mb(max_size),
    );
}

fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1_000_000.0
}

#[derive(Debug, Clone)]
pub(crate) struct CacheFile {
    pub path: PathBuf,
    pub size: u64,
    pub system_time: SystemTime,
}

fn collect_files(root: &Path) -> Vec<CacheFile> {
    if !root.exists() {
        return Vec::new();
    }

    WalkDir::new(root)
        .follow_root_links(false)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            let system_time = meta
                .accessed()
                .or_else(|_| meta.modified())
                .or_else(|_| meta.created())
                .unwrap_or(SystemTime::UNIX_EPOCH);

            Some(CacheFile {
                path: entry.path().to_path_buf(),
                size: meta.len(),
                system_time,
            })
        })
        .collect()
}

fn files_to_remove(
    files: &mut Vec<CacheFile>,
    max_size: u64,
    protected_path: &Path,
) -> Vec<CacheFile> {
    let total_size: u64 = files.iter().map(|f| f.size).sum();

    if total_size <= max_size {
        return Vec::new();
    }

    files.sort_by_key(|f| Reverse(f.system_time));

    let target =
        (TRIM_TARGET_NUMERATOR * max_size).div_ceil(TRIM_TARGET_DENOMINATOR);

    let mut remaining = total_size;
    let mut to_remove = Vec::new();

    while remaining > target {
        let Some(file) = files.pop() else { break };

        if file.path == protected_path {
            continue;
        }

        remaining = remaining.saturating_sub(file.size);
        to_remove.push(file);
    }

    to_remove
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    use super::{CacheFile, files_to_remove, should_trim};

    fn make_file(name: &str, size: u64, age_secs: u64) -> CacheFile {
        CacheFile {
            path: PathBuf::from(name),
            size,
            system_time: SystemTime::UNIX_EPOCH + Duration::from_secs(age_secs),
        }
    }

    #[test]
    fn trim_on_first_save() {
        assert!(should_trim(0, 100, 1, 10, true));
    }

    #[test]
    fn trim_when_over_limit() {
        assert!(should_trim(101, 100, 5, 10, false));
    }

    #[test]
    fn trim_at_interval() {
        assert!(should_trim(0, 100, 10, 10, false));
    }

    #[test]
    fn no_trim_under_limit_not_interval() {
        assert!(!should_trim(50, 100, 7, 10, false));
    }

    #[test]
    fn no_trim_when_interval_zero() {
        assert!(!should_trim(50, 100, 10, 0, false));
    }

    #[test]
    fn removes_oldest_until_under_target() {
        let mut files = vec![
            make_file("a", 5, 1), // oldest
            make_file("b", 5, 2),
            make_file("c", 5, 3), // newest
        ];

        let removed = files_to_remove(&mut files, 10, Path::new("none"));

        assert_eq!(removed.len(), 2);
        assert_eq!(removed[0].path, PathBuf::from("a"));
        assert_eq!(removed[1].path, PathBuf::from("b"));
    }

    #[test]
    fn returns_empty_when_under_limit() {
        let mut files = vec![make_file("a", 5, 1)];
        let removed = files_to_remove(&mut files, 10, Path::new("none"));
        assert!(removed.is_empty());
    }

    #[test]
    fn skips_protected_path() {
        let mut files = vec![
            make_file("a", 5, 1), // oldest, protected
            make_file("b", 5, 2),
            make_file("c", 5, 3),
        ];

        let removed = files_to_remove(&mut files, 10, Path::new("a"));

        assert!(removed.iter().all(|f| f.path != PathBuf::from("a")));
        assert!(removed.iter().any(|f| f.path == PathBuf::from("b")));
    }

    #[test]
    fn trim_to_target_not_to_zero() {
        let mut files = vec![
            make_file("a", 4, 1),
            make_file("b", 4, 2),
            make_file("c", 4, 3),
            make_file("d", 4, 4),
        ];

        let removed = files_to_remove(&mut files, 12, Path::new("none"));
        let removed_bytes: u64 = removed.iter().map(|f| f.size).sum();

        assert_eq!(removed.len(), 2);
        assert!(removed_bytes >= 7);
    }
}
