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

const REDACTED: &str = "<redacted>";

/// Redact credential material from a raw IRC line before it is written to the
/// on-disk protocol log. Operates on the wire representation so it is
/// independent of message parsing and applies equally to sent and received
/// lines. Never alters the bytes actually sent on the wire.
fn redact_log_line(line: &str) -> Cow<'_, str> {
    let (body, eol) = split_eol(line);

    // Skip any leading IRCv3 message tags (@...) and source prefix (:...) --
    // neither is secret -- to reach the command that decides redaction.
    let mut rest = body.trim_start();
    let mut prefix = String::new();
    while rest.starts_with('@') || rest.starts_with(':') {
        let Some((head, tail)) = rest.split_once(' ') else {
            return Cow::Borrowed(line); // tags/prefix with no command
        };
        prefix.push_str(head);
        prefix.push(' ');
        rest = tail.trim_start();
    }

    let Some((command, args)) = rest.split_once(' ') else {
        return Cow::Borrowed(line); // command with no arguments
    };

    let redacted_args = match command.to_ascii_uppercase().as_str() {
        "PASS" => Some(REDACTED.to_string()),
        "OPER" => Some(redact_after_first(args)),
        "AUTHENTICATE" => redact_authenticate(args),
        "PRIVMSG" => redact_services_message(args),
        _ => None,
    };

    match redacted_args {
        Some(redacted) => {
            Cow::Owned(format!("{prefix}{command} {redacted}{eol}"))
        }
        None => Cow::Borrowed(line),
    }
}

fn split_eol(line: &str) -> (&str, &str) {
    if let Some(rest) = line.strip_suffix("\r\n") {
        (rest, "\r\n")
    } else if let Some(rest) = line.strip_suffix('\n') {
        (rest, "\n")
    } else {
        (line, "")
    }
}

// Keep the first argument (e.g. an OPER name), redact everything after it.
fn redact_after_first(args: &str) -> String {
    match args.split_once(' ') {
        Some((first, _)) => format!("{first} {REDACTED}"),
        None => REDACTED.to_string(),
    }
}

// AUTHENTICATE carries either a SASL control token / mechanism name (not
// secret) or a base64 credential payload (secret). Redact anything that is not
// a control token or an uppercase mechanism identifier.
fn redact_authenticate(args: &str) -> Option<String> {
    let payload = args.trim();
    if payload == "+" || payload == "*" {
        return None;
    }
    let is_mechanism = !payload.is_empty()
        && payload
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-');
    if is_mechanism {
        None
    } else {
        Some(REDACTED.to_string())
    }
}

// NickServ-style authentication sent as a PRIVMSG, e.g.
// `PRIVMSG NickServ :IDENTIFY hunter2`. Redact the credential following a known
// services auth keyword, keeping the target and keyword for context. Services
// auth always targets a nick (never a channel), which keeps ordinary channel
// messages that merely start with one of these words out of scope.
fn redact_services_message(args: &str) -> Option<String> {
    const SERVICE_KEYWORDS: &[&str] =
        &["IDENTIFY", "REGISTER", "SETPASS", "GHOST", "RELEASE"];

    let (target, message) = args.split_once(' ')?;
    if target.starts_with(['#', '&', '+', '!']) {
        return None;
    }

    let body = message.strip_prefix(':').unwrap_or(message);
    let (keyword, _rest) = body.split_once(' ')?;

    if SERVICE_KEYWORDS
        .iter()
        .any(|k| keyword.eq_ignore_ascii_case(k))
    {
        Some(format!("{target} :{keyword} {REDACTED}"))
    } else {
        None
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
                        redact_log_line(&decode_line(src)).into_owned(),
                    ));
                }

                return Err(Error::LineTooLong);
            }
            return Ok(None);
        };

        let bytes = src.split_to(pos + 2);
        let line = decode_line(&bytes);

        if let Some(logger) = &self.logger {
            let _ = logger
                .send(CodecLog::Received(redact_log_line(&line).into_owned()));
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
                redact_log_line(&String::from_utf8_lossy(&bytes)).into_owned(),
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

    #[test]
    fn redacts_credentials_in_protocol_log() {
        use super::redact_log_line;

        assert_eq!(redact_log_line("PASS hunter2\r\n"), "PASS <redacted>\r\n");
        assert_eq!(redact_log_line("PASS :hunter2\r\n"), "PASS <redacted>\r\n");
        assert_eq!(
            redact_log_line("OPER admin s3cret\r\n"),
            "OPER admin <redacted>\r\n"
        );
        // SASL: mechanism selection and control tokens are not secret.
        assert_eq!(
            redact_log_line("AUTHENTICATE PLAIN\r\n"),
            "AUTHENTICATE PLAIN\r\n"
        );
        assert_eq!(redact_log_line("AUTHENTICATE +\r\n"), "AUTHENTICATE +\r\n");
        // ...but a credential payload (contains lowercase / is not a bare
        // mechanism name) is.
        assert_eq!(
            redact_log_line("AUTHENTICATE placeholder-credential\r\n"),
            "AUTHENTICATE <redacted>\r\n"
        );
        // NickServ-style services authentication.
        assert_eq!(
            redact_log_line("PRIVMSG NickServ :IDENTIFY hunter2\r\n"),
            "PRIVMSG NickServ :IDENTIFY <redacted>\r\n"
        );
        assert_eq!(
            redact_log_line("PRIVMSG NickServ :identify acct hunter2\r\n"),
            "PRIVMSG NickServ :identify <redacted>\r\n"
        );
        // Tags/source before the command are skipped, not treated as command.
        assert_eq!(
            redact_log_line("@time=x :srv PASS hunter2\r\n"),
            "@time=x :srv PASS <redacted>\r\n"
        );
    }

    #[test]
    fn preserves_non_sensitive_lines() {
        use super::redact_log_line;

        for line in [
            "PRIVMSG #chan :hello world\r\n",
            "JOIN #halloy\r\n",
            "NOTICE #chan :login page is down\r\n",
            "PING :12345\r\n",
        ] {
            assert_eq!(redact_log_line(line), line);
        }
    }
}
