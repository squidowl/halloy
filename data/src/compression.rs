use std::io;
use std::io::prelude::*;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub fn compress<W: Write, T: Serialize>(w: W, value: &T) -> Result<(), Error> {
    let encoder = GzEncoder::new(w, Compression::fast());
    serde_json::to_writer(encoder, &value).map_err(Error::Encode)
}

pub fn decompress<R: Read, T: DeserializeOwned>(rdr: R) -> Result<T, Error> {
    let decoder = GzDecoder::new(rdr);
    serde_json::from_reader(decoder).map_err(Error::Decode)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("compression failed")]
    Compression(io::Error),
    #[error("decompression failed")]
    Decompression(io::Error),
    #[error("encoding failed")]
    Encode(serde_json::Error),
    #[error("decoding failed")]
    Decode(serde_json::Error),
}
