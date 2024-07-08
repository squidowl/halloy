use std::io::Cursor;

use data::audio;
use kira::{
    manager::{
        backend::{cpal::CpalBackend, DefaultBackend},
        AudioManager, AudioManagerSettings,
    },
    sound::static_sound::StaticSoundData,
};

pub enum State {
    Ready { manager: AudioManager<CpalBackend> },
    Unsupported,
}

impl State {
    pub fn new() -> Self {
        let Ok(manager) = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
        else {
            return Self::Unsupported;
        };

        State::Ready { manager }
    }

    pub fn play(&mut self, sound: &audio::Sound) -> Result<(), PlayError> {
        if let State::Ready { manager } = self {
            let data = match sound {
                audio::Sound::Internal(internal) => {
                    let bytes = internal.bytes();
                    StaticSoundData::from_cursor(Cursor::new(bytes))?
                }
                audio::Sound::External(sound_name) => {
                    let Some(sound_path) = data::audio::find_external_sound(sound_name) else {
                        return Err(PlayError::NoSoundFound);
                    };

                    StaticSoundData::from_file(sound_path)?
                }
            };

            let _ = manager.play(data);

            Ok(())
        } else {
            Err(PlayError::Unsupported)
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlayError {
    #[error(transparent)]
    File(#[from] kira::sound::FromFileError),
    #[error("sound was not found")]
    NoSoundFound,
    #[error("unsupported")]
    Unsupported,
}
