use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek};
use std::path::PathBuf;
use std::time::Duration;

use futures::{Stream, StreamExt, stream};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageDecoder, Limits, RgbaImage};
use tokio::sync::{Semaphore, mpsc};
use tokio_stream::wrappers::ReceiverStream;

static DECODER: Semaphore = Semaphore::const_new(1);
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_FRAME_BYTES: u64 = 8 * 1024 * 1024;
const MIN_FRAME_DELAY: Duration = Duration::from_millis(20);

pub struct Frame {
    pub pixels: RgbaImage,
    pub delay: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Gif(#[from] gif::DecodingError),
    #[error("GIF is too large to animate")]
    TooLarge,
    #[error("GIF has no frames")]
    Empty,
    #[error("GIF decoder failed: {0}")]
    Worker(#[from] tokio::task::JoinError),
}

pub fn frames(path: PathBuf) -> impl Stream<Item = Result<Frame, Error>> {
    stream::once(async move {
        let (sender, receiver) = mpsc::channel(1);

        tokio::spawn(async move {
            let permit = tokio::select! {
                () = sender.closed() => return,
                permit = DECODER.acquire() => permit,
            };
            let Ok(permit) = permit else {
                return;
            };
            let output = sender.clone();
            let result = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                if output.is_closed() {
                    return Ok(());
                }

                let file = File::open(path)?;
                if file.metadata()?.len() > MAX_FILE_BYTES {
                    return Err(Error::TooLarge);
                }

                decode(
                    &mut BufReader::new(file),
                    || output.is_closed(),
                    |frame| output.blocking_send(Ok(frame)).is_ok(),
                )
            })
            .await
            .unwrap_or_else(|error| Err(Error::Worker(error)));

            if let Err(error) = result {
                let _ = sender.send(Err(error)).await;
            }
        });

        ReceiverStream::new(receiver)
    })
    .flatten()
}

fn decode(
    reader: &mut (impl BufRead + Seek),
    cancelled: impl Fn() -> bool,
    mut emit: impl FnMut(Frame) -> bool,
) -> Result<(), Error> {
    if cancelled() {
        return Ok(());
    }
    let mut options = gif::DecodeOptions::new();
    options.set_memory_limit(gif::MemoryLimit::Bytes(
        MAX_FRAME_BYTES.try_into().map_err(|_| Error::TooLarge)?,
    ));
    let mut repeat = options.read_info(&mut *reader)?.repeat();

    loop {
        if cancelled() {
            return Ok(());
        }

        reader.rewind()?;
        let mut decoder = GifDecoder::new(&mut *reader)?;
        if decoder.total_bytes() > MAX_FRAME_BYTES {
            return Err(Error::TooLarge);
        }

        let mut limits = Limits::default();
        limits.max_alloc = Some(3 * MAX_FRAME_BYTES);
        decoder.set_limits(limits)?;

        let mut frames = decoder.into_frames();
        let mut count = 0;
        loop {
            if cancelled() {
                return Ok(());
            }
            let Some(frame) = frames.next() else {
                break;
            };
            let frame = frame?;
            let delay = Duration::from(frame.delay()).max(MIN_FRAME_DELAY);
            if !emit(Frame {
                pixels: frame.into_buffer(),
                delay,
            }) {
                return Ok(());
            }
            count += 1;
        }

        if count == 0 {
            return Err(Error::Empty);
        }
        // No need to repeat a GIF with one frame.
        if count == 1 {
            return Ok(());
        }
        match &mut repeat {
            gif::Repeat::Finite(0) => return Ok(()),
            gif::Repeat::Finite(remaining) => *remaining -= 1,
            gif::Repeat::Infinite => {}
        }
    }
}
