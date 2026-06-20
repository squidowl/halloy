use std::io;

use bytes::BytesMut;
use encoding_rs::{ISO_2022_JP, UTF_8};
use proto::{Message, format, parse};
use tokio_util::codec::{Decoder, Encoder};

pub type ParseResult<T = Message, E = parse::Error> = std::result::Result<T, E>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Encoding {
    #[default]
    Utf8,
    Iso2022Jp,
}

impl Encoding {
    fn encoding_rs(self) -> &'static encoding_rs::Encoding {
        match self {
            Self::Utf8 => UTF_8,
            Self::Iso2022Jp => ISO_2022_JP,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Codec {
    encoding: Encoding,
}

impl Codec {
    pub fn new(encoding: Encoding) -> Self {
        Self { encoding }
    }
}

impl Decoder for Codec {
    type Item = ParseResult;
    type Error = Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let Some(pos) = src.windows(2).position(|b| b == [b'\r', b'\n']) else {
            return Ok(None);
        };

        let line = src.split_to(pos + 2);
        let (decoded, _, _) = self.encoding.encoding_rs().decode(&line);

        Ok(Some(parse::message(&decoded)))
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
        let (encoded, _, had_errors) =
            self.encoding.encoding_rs().encode(&encoded);

        if had_errors {
            return Err(Error::UnmappableCharacter(self.encoding));
        }

        if encoded.len() > format::BYTE_LIMIT {
            return Err(Error::ExceedsByteLimit {
                bytes: encoded.len(),
                bytes_limit: format::BYTE_LIMIT,
            });
        }

        dst.extend_from_slice(&encoded);

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("message contains characters that cannot be encoded as {0:?}")]
    UnmappableCharacter(Encoding),
    #[error(
        "message exceeds maximum encoded length ({bytes}/{bytes_limit} bytes)"
    )]
    ExceedsByteLimit { bytes: usize, bytes_limit: usize },
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use proto::command;
    use tokio_util::codec::{Decoder, Encoder};

    use super::{Codec, Encoding, Error};

    #[test]
    fn decodes_iso_2022_jp_messages() {
        let mut codec = Codec::new(Encoding::Iso2022Jp);
        let mut input = BytesMut::from(
            &b":alice!u@h PRIVMSG #test :\x1b$B$3$s$K$A$O\x1b(B\r\n"[..],
        );

        let message = codec.decode(&mut input).unwrap().unwrap().unwrap();

        assert_eq!(message.command.parameters()[1], "こんにちは");
    }

    #[test]
    fn encodes_iso_2022_jp_messages() {
        let mut codec = Codec::new(Encoding::Iso2022Jp);
        let mut output = BytesMut::new();

        codec
            .encode(command!("PRIVMSG", "#test", "こんにちは"), &mut output)
            .unwrap();

        assert_eq!(
            &output[..],
            b"PRIVMSG #test :\x1b$B$3$s$K$A$O\x1b(B\r\n"
        );
    }

    #[test]
    fn errors_on_unmappable_iso_2022_jp_characters() {
        let mut codec = Codec::new(Encoding::Iso2022Jp);
        let mut output = BytesMut::new();

        let error = codec
            .encode(command!("PRIVMSG", "#test", "hello \u{1f44b}"), &mut output)
            .unwrap_err();

        assert!(matches!(
            error,
            Error::UnmappableCharacter(Encoding::Iso2022Jp)
        ));
    }

    #[test]
    fn errors_on_encoded_messages_over_byte_limit() {
        let mut codec = Codec::new(Encoding::Iso2022Jp);
        let mut output = BytesMut::new();

        let error = codec
            .encode(command!("PRIVMSG", "#test", "あ".repeat(260)), &mut output)
            .unwrap_err();

        assert!(matches!(
            error,
            Error::ExceedsByteLimit {
                bytes: _,
                bytes_limit: 512
            }
        ));
    }
}
