use std::env;
use std::path::{Path, PathBuf};

pub fn config_dir() -> Option<PathBuf> {
    // HOST_* checked first for flatpak
    #[cfg(target_os = "linux")]
    {
        env::var("HOST_XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .filter(|p| is_absolute(p))
            .or_else(dirs_next::config_dir)
    }

    #[cfg(not(target_os = "linux"))]
    {
        dirs_next::config_dir()
    }
}

pub fn data_dir() -> Option<PathBuf> {
    // HOST_* checked first for flatpak
    #[cfg(target_os = "linux")]
    {
        env::var("HOST_XDG_DATA_HOME")
            .ok()
            .map(PathBuf::from)
            .filter(|p| is_absolute(p))
            .or_else(dirs_next::data_dir)
    }

    #[cfg(not(target_os = "linux"))]
    {
        dirs_next::data_dir()
    }
}

fn is_absolute(path: &Path) -> bool {
    path.is_absolute()
}
