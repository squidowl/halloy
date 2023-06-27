use std::env;
use std::path::PathBuf;

pub const VERSION: &str = env!("VERSION");
pub const GIT_HASH: Option<&str> = option_env!("GIT_HASH");

pub fn formatted_version() -> String {
    let hash = GIT_HASH
        .map(|hash| format!(" ({hash})"))
        .unwrap_or_default();

    format!("{}{hash}", VERSION)
}

pub(crate) fn config_dir() -> PathBuf {
    // HOST_* checked first for flatpak
    #[cfg(target_os = "linux")]
    {
        env::var("HOST_XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .filter(|p| is_absolute(p))
            .or_else(dirs_next::config_dir)
            .expect("expected valid config dir")
    }

    #[cfg(not(target_os = "linux"))]
    {
        dirs_next::config_dir().expect("expected valid config dir")
    }
}

pub(crate) fn data_dir() -> Option<PathBuf> {
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

#[cfg(target_os = "linux")]
fn is_absolute(path: &std::path::Path) -> bool {
    path.is_absolute()
}
