use std::path::PathBuf;
use std::{io::Cursor, sync::Arc};

use kira::{
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings, DefaultBackend},
    sound::static_sound::StaticSoundData,
};
use serde::Deserialize;

use crate::Config;

pub struct Manager(AudioManager<CpalBackend>);

impl Manager {
    pub fn new() -> Result<Self, InitializationError> {
        AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map(Self)
            .map_err(|_| InitializationError::Unsupported)
    }

    pub fn play(&mut self, sound: &Sound) -> Result<(), PlayError> {
        self.0
            .play(sound.data.clone())
            .map_err(|_| PlayError::PlaySoundError)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Sound {
    data: StaticSoundData,
}

impl Sound {
    pub fn load(name: &str) -> Result<Sound, LoadError> {
        let data = if let Ok(internal) = Internal::try_from(name) {
            StaticSoundData::from_cursor(Cursor::new(internal.bytes()))?
        } else {
            let Some(sound_path) = find_external_sound(name) else {
                return Err(LoadError::NoSoundFound);
            };

            StaticSoundData::from_file(sound_path)?
        };

        Ok(Sound { data })
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
            Internal::Squeak => include_bytes!("../../sounds/squeak.ogg").to_vec(),
            Internal::Whistle => include_bytes!("../../sounds/whistle.ogg").to_vec(),
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

fn find_external_sound(sound: &str) -> Option<PathBuf> {
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

#[derive(Debug, thiserror::Error)]
pub enum PlayError {
    #[error("error occured when playing a sound")]
    PlaySoundError,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    File(Arc<kira::sound::FromFileError>),
    #[error("sound was not found")]
    NoSoundFound,
}

impl From<kira::sound::FromFileError> for LoadError {
    fn from(error: kira::sound::FromFileError) -> Self {
        Self::File(Arc::new(error))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InitializationError {
    #[error("unsupported")]
    Unsupported,
}
