use std::env;
use std::path::PathBuf;

pub const VERSION: &str = env!("VERSION");
pub const GIT_HASH: Option<&str> = option_env!("GIT_HASH");
pub const CONFIG_FILE_NAME: &str = "config.yaml";
pub const APPLICATION_ID: &str = "org.squidowl.halloy";

pub fn formatted_version() -> String {
    let hash = GIT_HASH
        .map(|hash| format!(" ({hash})"))
        .unwrap_or_default();

    format!("{}{hash}", VERSION)
}

pub fn config_dir() -> PathBuf {
    portable_dir().unwrap_or_else(|| {
        dirs_next::config_dir()
            .expect("expected valid config dir")
            .join("halloy")
    })
}

pub fn data_dir() -> PathBuf {
    portable_dir().unwrap_or_else(|| {
        dirs_next::data_dir()
            .expect("expected valid data dir")
            .join("halloy")
    })
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
