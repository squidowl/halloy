use std::fs::read;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;

use crate::Config;

#[derive(Debug, Clone)]
pub struct Sound(Arc<Vec<u8>>);

impl AsRef<[u8]> for Sound {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Sound {
    pub fn load(name: &str) -> Result<Sound, LoadError> {
        let source = if let Ok(internal) = Internal::try_from(name) {
            internal.bytes()
        } else {
            let sound_path = find_external_sound(name)?;

            read(sound_path)?
        };

        Ok(Sound(Arc::new(source)))
    }
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
            Internal::Squeak => {
                include_bytes!("../../sounds/squeak.ogg").to_vec()
            }
            Internal::Whistle => {
                include_bytes!("../../sounds/whistle.ogg").to_vec()
            }
            Internal::Bonk => include_bytes!("../../sounds/bonk.ogg").to_vec(),
            Internal::Sing => include_bytes!("../../sounds/sing.ogg").to_vec(),
        }
    }
}

impl TryFrom<&str> for Internal {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "dong" => Ok(Self::Dong),
            "peck" => Ok(Self::Peck),
            "ring" => Ok(Self::Ring),
            "squeak" => Ok(Self::Squeak),
            "whistle" => Ok(Self::Whistle),
            "bonk" => Ok(Self::Bonk),
            "sing" => Ok(Self::Sing),
            _ => Err(()),
        }
    }
}

fn find_external_sound(sound: &str) -> Result<PathBuf, LoadError> {
    let sounds_dir = Config::sounds_dir();

    for e in walkdir::WalkDir::new(sounds_dir.clone())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if e.metadata().is_ok_and(|data| data.is_file())
            && e.file_name() == sound
        {
            return Ok(e.path().to_path_buf());
        }
    }

    let sounds_dir =
        if let Ok(sounds_dir) = sounds_dir.into_os_string().into_string() {
            format!(" in {sounds_dir}")
        } else {
            String::new()
        };

    Err(LoadError::NoSoundFound(sound.to_string(), sounds_dir))
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    File(Arc<std::io::Error>),
    #[error("sound \"{0}\" was not found{1}")]
    NoSoundFound(String, String),
}

impl From<std::io::Error> for LoadError {
    fn from(error: std::io::Error) -> Self {
        Self::File(Arc::new(error))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InitializationError {
    #[error("unsupported")]
    Unsupported,
}
