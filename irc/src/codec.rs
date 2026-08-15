use std::borrow::Cow;
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

/// Decodes an IRC line as UTF-8, falling back to ISO-8859-1 when needed.
///
/// Keeping wire decoding in the codec lets a connection select a different
/// encoding without making the IRC message parser configuration-aware.
fn decode_line(bytes: &[u8]) -> Cow<'_, str> {
    match str::from_utf8(bytes) {
        Ok(utf8) => Cow::Borrowed(utf8),
        Err(_) => Cow::Owned(bytes.iter().map(|&byte| byte as char).collect()),
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
                        decode_line(src).into_owned(),
                    ));
                }

                return Err(Error::LineTooLong);
            }
            return Ok(None);
        };

        let bytes = src.split_to(pos + 2);
        let line = decode_line(&bytes);

        if let Some(logger) = &self.logger {
            let _ = logger.send(CodecLog::Received(line.to_string()));
        }

        Ok(Some(parse::message(&line)))
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

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use proto::Command;
    use tokio_util::codec::Decoder;

    use super::Codec;

    fn decode(input: &[u8]) -> proto::Message {
        let mut codec = Codec::new(None);
        let mut input = BytesMut::from(input);

        codec.decode(&mut input).unwrap().unwrap().unwrap()
    }

    #[test]
    fn iso_8859_1_fallback() {
        let mut input = b"PRIVMSG #chan :moi ".to_vec();
        input.extend_from_slice(&[0xE4, 0xE4]);
        input.extend_from_slice(b"\r\n");

        let message = decode(&input);

        assert_eq!(
            message.command,
            Command::PRIVMSG("#chan".into(), "moi ää".into())
        );
    }

    #[test]
    fn utf8_is_preferred() {
        let message = decode("PRIVMSG #chan :moi ää\r\n".as_bytes());

        assert_eq!(
            message.command,
            Command::PRIVMSG("#chan".into(), "moi ää".into())
        );
    }

    #[test]
    fn iso_8859_1_fallback_applies_to_the_entire_line() {
        let message = decode(
            b"@id=invalid\x80utf8 :dan!d@localhost PRIVMSG #chan :Hello \xF0\x90\x80World\r\n",
        );

        assert_eq!(
            message.tags.get("id").map(String::as_str),
            Some("invalid\u{80}utf8")
        );
        assert_eq!(
            message.command,
            Command::PRIVMSG("#chan".into(), "Hello ð\u{90}\u{80}World".into())
        );
    }

    #[test]
    fn iso_8859_1_fallback_decodes_incomplete_utf8_sequences() {
        let message = decode(
            b":dan!d@localhost PART #halloy :My utf8 is br\xF4\x91\x87ken\r\n",
        );

        assert_eq!(
            message.command,
            Command::PART(
                "#halloy".into(),
                Some("My utf8 is brô\u{91}\u{87}ken".into())
            )
        );
    }
}
