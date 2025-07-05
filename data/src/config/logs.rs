use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct Logs {
    #[serde(default = "default_file_level")]
    pub file_level: LevelFilter,
    #[serde(
        default = "default_pane_level",
        deserialize_with = "restricted_deserialize_level_filter"
    )]
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

fn restricted_deserialize_level_filter<'de, D>(
    deserializer: D,
) -> Result<LevelFilter, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum Data {
        Off,
        Error,
        Warn,
        Info,
        Debug,
    }

    Ok(match Data::deserialize(deserializer)? {
        Data::Off => LevelFilter::Off,
        Data::Error => LevelFilter::Error,
        Data::Warn => LevelFilter::Warn,
        Data::Info => LevelFilter::Info,
        Data::Debug => LevelFilter::Debug,
    })
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
