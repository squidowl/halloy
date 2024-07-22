use std::{io::Cursor, sync::Arc, thread};

use data::audio::Sound;
use rodio::{Decoder, OutputStream, Sink};

pub fn play(sound: Sound) {
    thread::spawn(move || {
        if let Err(e) = _play(sound) {
            log::error!("Failed to play sound: {e}");
        }
    });
}

fn _play(sound: Sound) -> Result<(), PlayError> {
    let (_stream, stream_handle) = OutputStream::try_default()?;

    let sink = Sink::try_new(&stream_handle)?;

    let source = Decoder::new(Cursor::new(sound))?;

    sink.append(source);

    sink.sleep_until_end();

    Ok(())
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
