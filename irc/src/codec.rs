use std::io;

use bytes::BytesMut;
use proto::{Message, format, parse};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::codec::{Decoder, Encoder};

pub type ParseResult<T = Message, E = parse::Error> = std::result::Result<T, E>;

/// Maximum bytes buffered for a single IRC line before it is rejected.
/// Generous headroom over the / IRCv3 message-tags limit (8191;
/// https://ircv3.net/specs/extensions/message-tags#size-limit) plus the
/// 512-byte message.
const MAX_LINE_LENGTH: usize = 16 * 1024;

pub struct Codec {
    logger: Option<UnboundedSender<CodecLog>>,
    encoding: &'static encoding_rs::Encoding,
}

impl Codec {
    pub fn new(
        logger: Option<UnboundedSender<CodecLog>>,
        encoding: &'static encoding_rs::Encoding,
    ) -> Self {
        Self { logger, encoding }
    }
}

impl Decoder for Codec {
    type Item = ParseResult;
    type Error = Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let Some(pos) = src.windows(2).position(|b| b == *b"\r\n") else {
            // Guard against a peer that never sends CRLF: without a cap the framed stream would
            // buffer the "line" without bound, exhausting memory from a single connection.
            if src.len() > MAX_LINE_LENGTH {
                if let Some(logger) = &self.logger {
                    let (decoded, _, _) = self.encoding.decode(src);
                    let _ =
                        logger.send(CodecLog::Received(decoded.into_owned()));
                }

                return Err(Error::LineTooLong);
            }
            return Ok(None);
        };

        let bytes = src.split_to(pos + 2);

        let (decoded, _, _) = self.encoding.decode(&bytes);
        let decoded = decoded.into_owned();

        if let Some(logger) = &self.logger {
            let _ = logger.send(CodecLog::Received(decoded.clone()));
        }

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

        let (bytes, _, _) = self.encoding.encode(&encoded);
        dst.extend(bytes.as_ref());

        if let Some(logger) = &self.logger {
            let _ = logger.send(CodecLog::Sent(encoded));
        }

        Ok(())
    }
}

pub enum CodecLog {
    Received(String),
    Sent(String),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("IRC line exceeded the maximum buffered length")]
    LineTooLong,
}

#[cfg(test)]
mod tests {
    use proto::Command;

    use super::*;

    #[test]
    fn encodes_and_decodes_iso_8859_15() {
        let mut codec = Codec::new(None, encoding_rs::ISO_8859_15);

        let message: Message =
            Command::PRIVMSG("#canal".into(), "café à la euro €".into()).into();

        let mut buf = BytesMut::new();
        codec.encode(message.clone(), &mut buf).unwrap();

        // The euro sign is the tell: it's absent from ISO-8859-1 but present
        // in ISO-8859-15, and is encoded as a single byte, not the 3
        // UTF-8 bytes it would take otherwise.
        assert!(buf.contains(&0xA4));

        let decoded = codec.decode(&mut buf).unwrap().unwrap().unwrap();

        assert_eq!(decoded, message);
    }
}
