use kira::{
	manager::{backend::{cpal::CpalBackend, DefaultBackend}, AudioManager, AudioManagerSettings},
	sound::static_sound::{StaticSoundData, StaticSoundSettings},
};

pub struct Volume(f64);

impl Default for Volume {
    fn default() -> Volume {
        Volume(1.0)
    }
}

pub enum Audio {
    Ready {
        manager: AudioManager<CpalBackend>,
        volume: Volume,
    },
    Unsupported,
}

impl Audio {
    pub fn new() -> Self {
        let Ok(manager) = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) else {
            return Self::Unsupported;
        };

        Audio::Ready { manager, volume: Default::default() }
    }

    pub fn play(&mut self, sound: &str) -> Result<(), PlayError> {
        if let Audio::Ready { manager, volume } = self {
            // is sound external.

            // is sound internal
            let sounds = data::Config::sounds_dir();
            let Some(sound) = data::audio::find_sound(&sounds, sound) else {
                return Err(PlayError::NoSoundFound);
            };

            let settings = StaticSoundSettings::new();
            let data = StaticSoundData::from_file(sound, settings)?;
            let _ = manager.play(data);

            Ok(())

        } else {
            Err(PlayError::Unsupported)
        }
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