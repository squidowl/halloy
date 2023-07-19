use std::{io, string::FromUtf8Error};

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

struct Codec;

impl Decoder for Codec {
    type Item = codec::Message;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let Some(pos) = src.windows(2).position(|b| b == [b'\r', b'\n']) else {
            return Ok(None);
        };

        let bytes = src.split_to(pos + 2);
        let input = String::from_utf8(bytes.to_vec())?;

        Ok(Some(codec::parse::message(&input)?))
    }
}

impl Encoder<codec::Message> for Codec {
    type Error = Error;

    fn encode(&mut self, message: codec::Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let encoded = codec::format::message(message);

        dst.extend(encoded.into_bytes());

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("invalid utf-8 encoding")]
    InvalidUtf8(#[from] FromUtf8Error),
    #[error("parse error: {0}")]
    Parse(#[from] codec::parse::Error),
}
