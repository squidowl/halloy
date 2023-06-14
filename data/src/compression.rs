use std::io;
use std::io::prelude::*;

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn compress<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    let bytes = bincode::serialize(&value).map_err(Error::Encode)?;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&bytes).map_err(Error::Compression)?;
    Ok(encoder.finish().map_err(Error::Compression)?)
}

pub fn decompress<T: DeserializeOwned>(data: &[u8]) -> Result<T, Error> {
    let mut bytes = vec![];
    let mut encoder = ZlibDecoder::new(data);
    encoder
        .read_to_end(&mut bytes)
        .map_err(Error::Decompression)?;
    Ok(bincode::deserialize(&bytes).map_err(Error::Decode)?)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("compression failed")]
    Compression(io::Error),
    #[error("decompression failed")]
    Decompression(io::Error),
    #[error("encoding failed")]
    Encode(bincode::Error),
    #[error("decoding failed")]
    Decode(bincode::Error),
}
