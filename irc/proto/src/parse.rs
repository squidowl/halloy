use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, char, crlf, none_of, one_of, satisfy};
use nom::combinator::{
    complete, cut, map, opt, peek, recognize, value, verify,
};
use nom::multi::{
    many_m_n, many0, many0_count, many1, many1_count, separated_list1,
};
use nom::sequence::{preceded, terminated, tuple};
use nom::{Finish, IResult};

use crate::{Command, Message, Source, Tags, User};

pub fn message_bytes(bytes: Vec<u8>) -> Result<Message, Error> {
    let input = String::from_utf8_lossy(&bytes);
    message(&input)
}

/// Parses a single IRC message terminated by '\r\n`
pub fn message(input: &str) -> Result<Message, Error> {
    let mut message = cut(terminated(
        // '@' <tags> <SPACE>
        tuple((
            opt(preceded(char('@'), terminated(tags, space))),
            opt(source),
            command,
        )),
        // Discard addtl. \r if it exists, allow whitespace before
        preceded(many0(char(' ')), alt((preceded(char('\r'), crlf), crlf))),
    ));

    message(input)
        .finish()
        .map(|(_, (tags, source, command))| Message {
            tags: tags.unwrap_or_default().into_iter().collect(),
            source,
            command,
        })
        .map_err(|e| Error::Parse {
            input: input.to_string(),
            nom: e.to_string(),
        })
}

/// Parses a list of tag-encoded values into a list of tags
pub fn tagstr(input: &str) -> Result<Tags, Error> {
    complete(tags)(input)
        .finish()
        .map(|(_, tags)| tags.into_iter().collect())
        .map_err(|e| Error::Parse {
            input: input.to_string(),
            nom: e.to_string(),
        })
}

fn tags(input: &str) -> IResult<&str, Vec<(String, String)>> {
    let escaped_char = alt((
        value(';', tag(r"\:")),
        value(' ', tag(r"\s")),
        value('\\', tag(r"\\")),
        value('\r', tag(r"\r")),
        value('\n', tag(r"\n")),
        // drop escape char '\'
        preceded(char('\\'), none_of(r":s\rn ")),
    ));
    // <sequence of any escaped characters except NUL, CR, LF, semicolon (`;`) and SPACE>
    let escaped_value = map(
        terminated(
            many1(alt((escaped_char, none_of("\0\r\n;\\ ")))),
            // drop trailing escape char '\'
            opt(char('\\')),
        ),
        |value| value.into_iter().collect::<String>(),
    );
    // '+'
    let client_prefix = char('+');
    // [ <client_prefix> ] [ <vendor> '/' ] <sequence of letters, digits, hyphens (`-`)>
    let key = recognize(tuple((
        opt(client_prefix),
        opt(terminated(many1_count(none_of("/ ;=")), char('/'))),
        many1_count(satisfy(|c| c.is_ascii_alphanumeric() || c == '-')),
    )));
    // <key> ['=' <escaped value>]
    let tag = map(
        tuple((key, opt(preceded(char('='), escaped_value)))),
        |(key, value): (&str, _)| (key.to_string(), value.unwrap_or_default()),
    );
    // <tag> [';' <tag>]*
    separated_list1(char(';'), tag)(input)
}

fn source(input: &str) -> IResult<&str, Source> {
    // <servername> / <user>
    let source = alt((
        map(terminated(user, peek(space)), Source::User),
        // Default all non-valid users to server
        map(
            terminated(recognize(many1(none_of(" "))), peek(space)),
            |host| Source::Server(host.to_string()),
        ),
    ));
    // ':' <source> <SPACE>
    terminated(preceded(char(':'), source), space)(input)
}

fn command(input: &str) -> IResult<&str, Command> {
    // <sequence of any characters except NUL, CR, LF, colon (`:`) and SPACE>
    let nospcrlfcl = |input| recognize(many1_count(none_of("\0\r\n: ")))(input);
    // *( ":" / " " / nospcrlfcl )
    let trailing =
        recognize(many0_count(alt((tag(":"), tag(" "), nospcrlfcl))));
    // nospcrlfcl *( ":" / nospcrlfcl )
    let middle = recognize(tuple((
        nospcrlfcl,
        many0_count(alt((tag(":"), nospcrlfcl))),
    )));
    // *( SPACE middle ) [ SPACE ":" trailing ]
    let parameters = tuple((
        many0(preceded(space, middle)),
        opt(preceded(space, preceded(char(':'), trailing))),
    ));
    // letter* / 3digit
    let command = alt((
        alpha1,
        recognize(many_m_n(3, 3, satisfy(|c| c.is_ascii_digit()))),
    ));
    // <command> <parameters>
    let (input, (command, (leading, trailing))) =
        tuple((command, parameters))(input)?;

    let parameters = leading
        .into_iter()
        .chain(trailing)
        .map(String::from)
        .collect();

    Ok((input, Command::new(command, parameters)))
}

fn space(input: &str) -> IResult<&str, ()> {
    map(many1_count(char(' ')), |_| ())(input)
}

fn user(input: &str) -> IResult<&str, User> {
    // <sequence of any characters except NUL, CR, LF, and SPACE> and @
    let username = recognize(many1_count(none_of("\0\r\n @")));
    // "-", "[", "]", "\", "`", "_", "^", "{", "|", "}", "*", "/", "@"
    let special = |input| one_of("-[]\\`_^{|}*/@")(input);
    // *( <letter> | <number> | <special> )
    let strict_nick = recognize(many1_count(alt((
        satisfy(|c| c.is_ascii_alphanumeric()),
        special,
    ))));
    // Used by things like matrix bridge
    // Also includes `.` if `:` exists and terminated by `!`
    // this enables us to use `:` and `.` without falsely matching
    // and server IP or hostname
    let expanded_nick = verify(
        recognize(terminated(
            many1_count(alt((
                satisfy(|c| c.is_ascii_alphanumeric()),
                special,
                one_of(":."),
            ))),
            peek(char('!')),
        )),
        |s: &str| s.contains(':') && s.contains('.'),
    );
    let nickname = alt((expanded_nick, strict_nick));
    // Parse remainder after @ as hostname
    let hostname = recognize(many1_count(none_of(" ")));
    //( <nickname> [ "!" <user> ] [ "@" <host> ] )
    map(
        tuple((
            nickname,
            opt(preceded(char('!'), username)),
            opt(preceded(char('@'), hostname)),
        )),
        |(nickname, username, hostname): (&str, Option<&str>, Option<&str>)| {
            User {
                nickname: nickname.to_string(),
                username: username.map(ToString::to_string),
                hostname: hostname.map(ToString::to_string),
            }
        },
    )(input)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("parsing failed: {:?}", input)]
    Parse { input: String, nom: String },
}

#[cfg(test)]
mod test {
    use nom::combinator::all_consuming;

    use crate::command::Numeric::*;
    use crate::{Command, Message, Source, User};

    use crate::tags;

    #[test]
    fn user() {
        let tests = [
            "dan!d@localhost",
            "dan@id/network!d@remote.host",
            "test!test@5555:5555:0:55:5555:5555:5555:5555",
            "[asdf]!~asdf@user/asdf/x-5555555",
        ];

        for test in tests {
            all_consuming(super::user)(test).unwrap();
        }
    }

    #[test]
    fn source() {
        let tests = [
            (
                ":irc.example.com ",
                Source::Server("irc.example.com".into()),
            ),
            (
                ":dan!d@localhost ",
                Source::User(User {
                    nickname: "dan".into(),
                    username: Some("d".into()),
                    hostname: Some("localhost".into()),
                }),
            ),
            (
                ":dan@id/network!d@remote.host ",
                Source::User(User {
                    nickname: "dan@id/network".into(),
                    username: Some("d".into()),
                    hostname: Some("remote.host".into()),
                }),
            ),
            (
                ":atw.hu.quakenet.org ",
                Source::Server("atw.hu.quakenet.org".into()),
            ),
            (":*.freenode.net ", Source::Server("*.freenode.net".into())),
            (
                ":foobar/server!~foobar@555.555.555.555.abc.efg.com ",
                Source::User(User {
                    nickname: "foobar/server".into(),
                    username: Some("~foobar".into()),
                    hostname: Some("555.555.555.555.abc.efg.com".into()),
                }),
            ),
            (
                ":foo:matrix.org!foo@matrix.org ",
                Source::User(User {
                    nickname: "foo:matrix.org".into(),
                    username: Some("foo".into()),
                    hostname: Some("matrix.org".into()),
                }),
            ),
            (":1.1.1.1 ", Source::Server("1.1.1.1".to_string())),
            (":1111:FFFF::1 ", Source::Server("1111:FFFF::1".to_string())),
        ];

        for (test, expected) in tests {
            let (_, source) = super::source(test).unwrap();
            assert_eq!(source, expected);
        }
    }

    #[test]
    fn message() {
        let tests = [
            (
                Vec::from(b":irc.example.com CAP LS * :multi-prefix extended-join sasl\r\n"),
                Message {
                    tags: tags![],
                    source: Some(Source::Server("irc.example.com".to_string())),
                    command: Command::CAP(
                        Some("LS".to_string()),
                        "*".to_string(),
                        Some("multi-prefix extended-join sasl".to_string()),
                        None,
                    ),
                },
            ),
            (
                Vec::from(b"@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up! \r\n"),
                Message {
                    tags: tags!["id" => "234AB"],
                    source: Some(Source::User(User {
                        nickname: "dan".into(),
                        username: Some("d".into()),
                        hostname: Some("localhost".into()),
                    })),
                    command: Command::PRIVMSG(
                        "#chan".to_string(),
                        "Hey what's up! ".to_string(),
                    ),
                },
            ),
            (
                Vec::from(b"CAP REQ :sasl\r\n"),
                Message {
                    tags: tags![],
                    source: None,
                    command: Command::CAP(
                        Some("REQ".to_string()),
                        "sasl".to_string(),
                        None,
                        None,
                    ),
                },
            ),
            (
                Vec::from(b"@tag=as\\\\\\:\\sdf\\z\\ UNKNOWN\r\n"),
                Message {
                    tags: tags!["tag" => "as\\; dfz"],
                    source: None,
                    command: Command::Unknown("UNKNOWN".to_string(), vec![]),
                },
            ),
            (
                Vec::from(b"@+1.1.1.1/wi2-asef-1=as\\\\\\:\\sdf\\z\\ UNKNOWN\r\n"),
                Message {
                    tags: tags!["+1.1.1.1/wi2-asef-1" => "as\\; dfz"],
                    source: None,
                    command: Command::Unknown("UNKNOWN".to_string(), vec![]),
                },
            ),
            (
                Vec::from(b":test!test@5555:5555:0:55:5555:5555:5555:5555 396 test user/test :is now your visible host\r\n"),
                Message {
                    tags: tags![],
                    source: Some(Source::User(User {
                        nickname: "test".into(),
                        username: Some("test".into()),
                        hostname: Some(
                            "5555:5555:0:55:5555:5555:5555:5555".into(),
                        ),
                    })),
                    command: Command::Unknown(
                        "396".to_string(),
                        vec![
                            "test".to_string(),
                            "user/test".to_string(),
                            "is now your visible host".to_string(),
                        ],
                    ),
                },
            ),
            (
                Vec::from(b":atw.hu.quakenet.org 001 test :Welcome to the QuakeNet IRC Network, test\r\n"),
                Message {
                    tags: tags![],
                    source: Some(Source::Server(
                        "atw.hu.quakenet.org".to_string(),
                    )),
                    command: Command::Numeric(
                        RPL_WELCOME,
                        vec![
                            "test".to_string(),
                            "Welcome to the QuakeNet IRC Network, test"
                                .to_string(),
                        ],
                    ),
                },
            ),
            (
                Vec::from(b"@time=2023-07-20T21:19:11.000Z :chat!test@user/test/bot/chat PRIVMSG ##chat :\\_o< quack!\r\n"),
                Message {
                    tags: tags!["time" => "2023-07-20T21:19:11.000Z"],
                    source: Some(Source::User(User {
                        nickname: "chat".into(),
                        username: Some("test".into()),
                        hostname: Some("user/test/bot/chat".into()),
                    })),
                    command: Command::PRIVMSG(
                        "##chat".to_string(),
                        "\\_o< quack!".to_string(),
                    ),
                },
            ),
            // Extra \r sent by digitalirc
            (
                Vec::from(b"@batch=JQlhpjWY7SYaBPQtXAfUQh;msgid=UGnor4DBoafs6ge0UgsHF7-aVdnYMbjbdTf9eEHQmPKWA;time=2024-11-07T12:04:28.361Z :foo!~foo@F3FF3610.5A633F24.29800D3F.IP JOIN #pixelcove * :foo\r\r\n"),
                Message {
                    tags: tags![
                        "batch" => "JQlhpjWY7SYaBPQtXAfUQh",
                        "msgid" => "UGnor4DBoafs6ge0UgsHF7-aVdnYMbjbdTf9eEHQmPKWA",
                        "time" => "2024-11-07T12:04:28.361Z",
                    ],
                    source: Some(Source::User(User {
                        nickname: "foo".into(),
                        username: Some("~foo".into()),
                        hostname: Some("F3FF3610.5A633F24.29800D3F.IP".into()),
                    })),
                    command: Command::JOIN(
                        "#pixelcove".to_string(),
                        Some("*".to_string()),
                    ),
                },
            ),
            // Space between message and crlf sent by DejaToons
            (
                Vec::from(b"@batch=AhaatzFmHPzct87cyiyxk4;time=2025-01-15T22:54:02.123Z;msgid=pgON6bxXjG7unoKIYwC3aV-KPRYjZhmCa3ZReibvMIrgw :atarians.dejatoons.net MODE #test +nt \r\n"),
                Message {
                    tags: tags![
                        "batch" => "AhaatzFmHPzct87cyiyxk4",
                        "time" => "2025-01-15T22:54:02.123Z",
                        "msgid" => "pgON6bxXjG7unoKIYwC3aV-KPRYjZhmCa3ZReibvMIrgw",
                    ],
                    source: Some(Source::Server(
                        "atarians.dejatoons.net".to_string(),
                    )),
                    command: Command::MODE(
                        "#test".to_string(),
                        Some("+nt".to_string()),
                        Some(vec![]),
                    ),
                },
            ),
            (
                Vec::from(b":soju.bouncer FAIL * ACCOUNT_REQUIRED :Authentication required\r\n"),
                Message {
                    tags: tags![],
                    source: Some(Source::Server("soju.bouncer".to_string())),
                    command: Command::FAIL(
                        "*".to_string(),
                        "ACCOUNT_REQUIRED".to_string(),
                        None,
                        "Authentication required".to_string(),
                    ),
                },
            ),
            (
                Vec::from(b"@id=invalid\x80utf8 :dan!d@localhost PRIVMSG #chan :Hello \xF0\x90\x80World\r\n"),
                Message {
                    tags: tags!["id" => "invalid�utf8"],
                    source: Some(Source::User(User {
                        nickname: "dan".into(),
                        username: Some("d".into()),
                        hostname: Some("localhost".into()),
                    })),
                    command: Command::PRIVMSG(
                        "#chan".to_string(),
                        "Hello �World".to_string(),
                    ),
                },
            ),
            (
                Vec::from(b":dan!d@localhost PART #halloy :My utf8 is br\xF4\x91\x87ken\r\n"),
                Message {
                    tags: tags![],
                    source: Some(Source::User(User {
                        nickname: "dan".into(),
                        username: Some("d".into()),
                        hostname: Some("localhost".into()),
                    })),
                    command: Command::PART(
                        "#halloy".to_string(),
                        Some("My utf8 is br���ken".to_string()),
                    ),
                },
            ),
        ];

        for (test, expected) in tests {
            let message = super::message_bytes(test).unwrap();
            assert_eq!(message, expected);
        }
    }

    #[test]
    fn tagstrs() {
        // source: https://codeberg.org/emersion/soju/src/branch/master/doc/ext/bouncer-networks.md#examples
        let tests = [
            (
                "name=OFTC;host=irc.oftc.net",
                tags![
                    "name" => "OFTC",
                    "host" => "irc.oftc.net",
                ],
            ),
            (
                r"name=My\sAwesome\sNetwork;state=disconnected",
                tags![
                        "name" => "My Awesome Network",
                        "state" => "disconnected",
                ],
            ),
        ];

        for (test, expected) in tests {
            let message = super::tagstr(test).unwrap();
            assert_eq!(message, expected);
        }
    }
}
