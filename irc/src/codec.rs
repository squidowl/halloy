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
}

impl Codec {
    pub fn new(logger: Option<UnboundedSender<CodecLog>>) -> Self {
        Self { logger }
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
                    let _ = logger.send(CodecLog::Received(
                        String::from_utf8_lossy(src).to_string(),
                    ));
                }

                return Err(Error::LineTooLong);
            }
            return Ok(None);
        };

        let bytes = src.split_to(pos + 2);

        if let Some(logger) = &self.logger {
            let _ = logger.send(CodecLog::Received(
                String::from_utf8_lossy(&bytes).to_string(),
            ));
        }

        Ok(Some(parse::message_bytes(&bytes)))
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

        let bytes = encoded.into_bytes();

        if let Some(logger) = &self.logger {
            let _ = logger.send(CodecLog::Sent(
                String::from_utf8_lossy(&bytes).to_string(),
            ));
        }

        dst.extend(bytes);

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
