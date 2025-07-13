use std::env;
use std::path::PathBuf;

pub const VERSION: &str = env!("VERSION");
pub const GIT_HASH: Option<&str> = option_env!("GIT_HASH");
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const APPLICATION_ID: &str = "org.squidowl.halloy";
pub const WIKI_WEBSITE: &str = "https://halloy.chat";
pub const THEME_WEBSITE: &str = "https://themes.halloy.chat";
pub const MIGRATION_WEBSITE: &str =
    "https://halloy.chat/guides/migrating-from-yaml.html";
pub const RELEASE_WEBSITE: &str =
    "https://github.com/squidowl/halloy/releases/latest";
pub const SOURCE_WEBSITE: &str = "https://github.com/squidowl/halloy/";

pub fn formatted_version() -> String {
    let hash = GIT_HASH
        .map(|hash| format!(" ({hash})"))
        .unwrap_or_default();

    format!("{VERSION}{hash}")
}

pub fn config_dir() -> PathBuf {
    portable_dir().unwrap_or_else(platform_specific_config_dir)
}

pub fn data_dir() -> PathBuf {
    portable_dir().unwrap_or_else(|| {
        dirs_next::data_dir()
            .expect("expected valid data dir")
            .join("halloy")
    })
}

pub fn cache_dir() -> PathBuf {
    dirs_next::cache_dir()
        .expect("expected valid cache dir")
        .join("halloy")
}

/// Checks if a config file exists in the same directory as the executable.
/// If so, it'll use that directory for both config & data dirs.
fn portable_dir() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;

    dir.join(CONFIG_FILE_NAME)
        .is_file()
        .then(|| dir.to_path_buf())
}

fn platform_specific_config_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        xdg_config_dir().unwrap_or_else(|| {
            dirs_next::config_dir()
                .expect("expected valid config dir")
                .join("halloy")
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs_next::config_dir()
            .expect("expected valid config dir")
            .join("halloy")
    }
}

#[cfg(target_os = "macos")]
fn xdg_config_dir() -> Option<PathBuf> {
    xdg::BaseDirectories::with_prefix("halloy")
        .find_config_file(CONFIG_FILE_NAME)
}
