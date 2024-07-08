use std::path::PathBuf;

use serde::Deserialize;

use crate::Config;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sound {
    Internal(Internal),
    External(String),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Internal {
    Dong,
    Peck,
    Ring,
    Squeak,
    Whistle,
    Bonk,
    Sing,
}

impl Internal {
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            Internal::Dong => include_bytes!("../../sounds/dong.ogg").to_vec(),
            Internal::Peck => include_bytes!("../../sounds/peck.ogg").to_vec(),
            Internal::Ring => include_bytes!("../../sounds/ring.ogg").to_vec(),
            Internal::Squeak => include_bytes!("../../sounds/squeak.ogg").to_vec(),
            Internal::Whistle => include_bytes!("../../sounds/whistle.ogg").to_vec(),
            Internal::Bonk => include_bytes!("../../sounds/bonk.ogg").to_vec(),
            Internal::Sing => include_bytes!("../../sounds/sing.ogg").to_vec(),
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
