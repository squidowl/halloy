use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Logs {
    #[serde(default = "default_file_level")]
    pub file_level: LevelFilter,
    #[serde(default = "default_pane_level")]
    pub pane_level: LevelFilter,
}

impl Default for Logs {
    fn default() -> Self {
        Self {
            file_level: default_file_level(),
            pane_level: default_pane_level(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LevelFilter {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LevelFilter> for log::LevelFilter {
    fn from(level: LevelFilter) -> Self {
        match level {
            LevelFilter::Off => log::LevelFilter::Off,
            LevelFilter::Error => log::LevelFilter::Error,
            LevelFilter::Warn => log::LevelFilter::Warn,
            LevelFilter::Info => log::LevelFilter::Info,
            LevelFilter::Debug => log::LevelFilter::Debug,
            LevelFilter::Trace => log::LevelFilter::Trace,
        }
    }
}

fn default_file_level() -> LevelFilter {
    LevelFilter::Debug
}

fn default_pane_level() -> LevelFilter {
    LevelFilter::Info
}
