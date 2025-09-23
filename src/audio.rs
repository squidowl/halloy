use std::io::Cursor;
use std::sync::Arc;
use std::thread;

use data::audio::Sound;
use rodio::{Decoder, OutputStreamBuilder, Sink};

pub fn play(sound: Sound) {
    thread::spawn(move || {
        if let Err(e) = _play(sound) {
            log::error!("Failed to play sound: {e}");
        }
    });
}

fn _play(sound: Sound) -> Result<(), PlayError> {
    let mut stream_handle = OutputStreamBuilder::open_default_stream()?;
    stream_handle.log_on_drop(false);
    let sink = Sink::connect_new(stream_handle.mixer());

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
