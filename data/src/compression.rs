use std::io;

use async_compression::tokio::bufread::GzipDecoder;
use async_compression::tokio::write::GzipEncoder;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, AsyncWriteExt as _};

pub async fn compress<R, W>(mut reader: R, writer: W) -> Result<(), Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut encoder = GzipEncoder::new(writer);
    tokio::io::copy(&mut reader, &mut encoder)
        .await
        .map_err(Error::Io)?;
    encoder.shutdown().await.map_err(Error::Compression)?;
    Ok(())
}

pub async fn decompress<R, W>(reader: R, mut writer: W) -> Result<(), Error>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut decoder = GzipDecoder::new(reader);
    tokio::io::copy(&mut decoder, &mut writer)
        .await
        .map_err(Error::Io)?;
    Ok(())
}

pub async fn serialize_and_compress<T, W>(
    value: &T,
    writer: W,
) -> Result<(), Error>
where
    T: ?Sized + Serialize,
    W: AsyncWrite + Unpin,
{
    let serialized = serde_json::to_vec(value).map_err(Error::Encode)?;
    compress(&serialized[..], writer).await?;
    Ok(())
}

pub async fn decompress_and_deserialize<T, R>(reader: R) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
    R: AsyncBufRead + Unpin,
{
    let mut decompressed = Vec::new();
    decompress(reader, &mut decompressed).await?;
    let value = serde_json::from_slice(&decompressed).map_err(Error::Decode)?;
    Ok(value)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O failed")]
    Io(io::Error),
    #[error("compression failed")]
    Compression(io::Error),
    #[error("decompression failed")]
    Decompression(io::Error),
    #[error("encoding failed")]
    Encode(serde_json::Error),
    #[error("decoding failed")]
    Decode(serde_json::Error),
}
