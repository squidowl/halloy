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
    dirs_next::config_dir().expect("expected valid config dir")
}

pub(crate) fn data_dir() -> PathBuf {
    dirs_next::data_dir().expect("expected valid data dir")
}
