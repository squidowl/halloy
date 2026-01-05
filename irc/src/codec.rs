use std::io;

use bytes::BytesMut;
use proto::{Message, format, parse};
use tokio_util::codec::{Decoder, Encoder};

pub type ParseResult<T = Message, E = parse::Error> = std::result::Result<T, E>;

pub struct Codec;

impl Decoder for Codec {
    type Item = ParseResult;
    type Error = Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let Some(pos) = src.windows(2).enumerate().find_map(|(i, b)| {
            if b == [b'\r', b'\n'] {
                Some(i + 2)
            } else if b[0] == b'\n' {
                Some(i + 1)
            } else {
                None
            }
        }) else {
            return Ok(None);
        };

        let bytes = Vec::from(src.split_to(pos));

        Ok(Some(parse::message_bytes(bytes)))
    }
}

impl Encoder<Message> for Codec {
    type Error = Error;

    fn encode(
        &mut self,
        message: Message,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        let encoded = format::message(message);

        dst.extend(encoded.into_bytes());

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
}
