use std::path::{Path, PathBuf};

pub fn find_sound(dir: &Path, sound: &str) -> Option<PathBuf> {
    for e in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if e.metadata().map(|data| data.is_file()).unwrap_or_default() {
            if e.file_name() == sound {
                return Some(e.path().to_path_buf())
            }
        }
    }

    None 
}
