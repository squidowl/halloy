use std::path::PathBuf;

use serde::Deserialize;

use crate::Config;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sound {
    Internal(Internal),
    External(String),
    None,
}

impl Default for Sound {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Internal {
    #[default]
    Halloy,
}

impl Internal {
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            Internal::Halloy => include_bytes!("../../sounds/classic.ogg").to_vec(),
        }
    }
}

pub fn find_external_sound(sound: &str) -> Option<PathBuf> {
    let sounds_dir = Config::sounds_dir();

    for e in walkdir::WalkDir::new(sounds_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if e.metadata().map(|data| data.is_file()).unwrap_or_default() && e.file_name() == sound {
            return Some(e.path().to_path_buf());
        }
    }

    None
}
