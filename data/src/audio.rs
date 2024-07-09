use std::io::Cursor;
use std::path::PathBuf;

use kira::{
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings, DefaultBackend},
    sound::static_sound::StaticSoundData,
};
use serde::{Deserialize, Deserializer};

use crate::Config;

pub struct Manager(AudioManager<CpalBackend>);

impl Manager {
    pub fn new() -> Result<Self, InitializationError> {
        AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map(Self)
            .map_err(|_| InitializationError::Unsupported)
    }

    pub fn play(&mut self, sound: &Sound) -> Result<(), PlayError> {
        if let Some(data) = sound.data.clone() {
            self.0.play(data).map_err(|_| PlayError::PlaySoundError)?;
            Ok(())
        } else {
            Err(PlayError::NoSoundData)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sound {
    name: String,
    pub data: Option<StaticSoundData>,
}

impl<'de> Deserialize<'de> for Sound {
    fn deserialize<D>(deserializer: D) -> Result<Sound, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name: String = serde::Deserialize::deserialize(deserializer)?;
        Ok(Sound { name, data: None })
    }
}

impl Sound {
    pub fn load_data(&mut self) -> Result<(), LoadError> {
        let data = if let Ok(internal) = Internal::try_from(self.name.as_str()) {
            StaticSoundData::from_cursor(Cursor::new(internal.bytes()))?
        } else {
            let Some(sound_path) = find_external_sound(self.name.as_str()) else {
                return Err(LoadError::NoSoundFound);
            };

            StaticSoundData::from_file(sound_path)?
        };

        self.data = Some(data);
        Ok(())
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
    #[error("sound has no data")]
    NoSoundData,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    File(#[from] kira::sound::FromFileError),
    #[error("sound was not found")]
    NoSoundFound,
}

#[derive(Debug, thiserror::Error)]
pub enum InitializationError {
    #[error("unsupported")]
    Unsupported,
}
