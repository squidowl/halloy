use std::fs::read;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

use rodio::{Decoder, OutputStream, Sink};
use serde::Deserialize;

use crate::Config;

#[derive(Debug, Clone)]
pub struct Sound(Vec<u8>);

impl Sound {
    pub fn load(name: &str) -> Result<Sound, LoadError> {
        let source = if let Ok(internal) = Internal::try_from(name) {
            internal.bytes()
        } else {
            let Some(sound_path) = find_external_sound(name) else {
                return Err(LoadError::NoSoundFound);
            };

            read(sound_path)?
        };

        Ok(Sound(source))
    }

    pub fn play(&self) -> Result<(), PlayError> {
        let (_stream, stream_handle) = OutputStream::try_default()?;

        let sink = Sink::try_new(&stream_handle)?;

        let source = Decoder::new(Cursor::new(self.0.clone()))?;

        sink.append(source);

        sink.sleep_until_end();

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
    #[error(transparent)]
    Decoding(Arc<rodio::decoder::DecoderError>),
    #[error(transparent)]
    Playing(Arc<rodio::PlayError>),
    #[error(transparent)]
    StreamInitialization(Arc<rodio::StreamError>),
}

impl From<rodio::decoder::DecoderError> for PlayError {
    fn from(error: rodio::decoder::DecoderError) -> Self {
        Self::Decoding(Arc::new(error))
    }
}

impl From<rodio::PlayError> for PlayError {
    fn from(error: rodio::PlayError) -> Self {
        Self::Playing(Arc::new(error))
    }
}

impl From<rodio::StreamError> for PlayError {
    fn from(error: rodio::StreamError) -> Self {
        Self::StreamInitialization(Arc::new(error))
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    File(Arc<std::io::Error>),
    #[error("sound was not found")]
    NoSoundFound,
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
