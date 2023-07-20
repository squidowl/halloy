use std::string::FromUtf8Error;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, char, crlf, none_of, one_of, satisfy};
use nom::combinator::{cut, map, opt, peek, recognize, value};
use nom::multi::{many0, many0_count, many1, many1_count, many_m_n, separated_list1};
use nom::sequence::{preceded, terminated, tuple};
use nom::{Finish, IResult};

use crate::{Command, Message, Source, Tag, User};

pub fn message_bytes(bytes: Vec<u8>) -> Result<Message, Error> {
    let input = String::from_utf8(bytes)?;
    message(&input)
}

/// Parses a single IRC message terminated by '\r\n`
pub fn message(input: &str) -> Result<Message, Error> {
    let mut message = cut(terminated(tuple((opt(tags), opt(source), command)), crlf));

    message(input)
        .finish()
        .map(|(_, (tags, source, command))| Message {
            tags: tags.unwrap_or_default(),
            source,
            command,
        })
        .map_err(|e| Error::Parse {
            input: input.to_string(),
            nom: e.to_string(),
        })
}

fn tags(input: &str) -> IResult<&str, Vec<Tag>> {
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
        opt(terminated(many1_count(none_of("/")), char('/'))),
        many1_count(satisfy(|c| c.is_ascii_alphanumeric() || c == '-')),
    )));
    // <key> ['=' <escaped value>]
    let tag = map(
        tuple((key, opt(preceded(char('='), escaped_value)))),
        |(key, value): (&str, _)| Tag {
            key: key.to_string(),
            value,
        },
    );
    // <tag> [';' <tag>]*
    let tags = separated_list1(char(';'), tag);
    // '@' <tags> <SPACE>
    preceded(char('@'), terminated(tags, space))(input)
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
    let trailing = recognize(many0_count(alt((tag(":"), tag(" "), nospcrlfcl))));
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
    let (input, (command, (leading, trailing))) = tuple((command, parameters))(input)?;

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
    // "-" "[", "]", "\", "`", "_", "^", "{", "|", "}"
    let special = one_of("-[]\\`_^{|}");
    // *( <letter> | <number> | <special> )
    let nickname = recognize(many1_count(alt((
        satisfy(|c| c.is_ascii_alphanumeric()),
        special,
    ))));
    // Parse remainder after @ as hostname
    let hostname = recognize(many1_count(none_of(" ")));
    //( <nickname> [ "!" <user> ] [ "@" <host> ] )
    map(
        tuple((
            nickname,
            opt(preceded(char('!'), username)),
            opt(preceded(char('@'), hostname)),
        )),
        |(nickname, username, hostname): (&str, Option<&str>, Option<&str>)| User {
            nickname: nickname.to_string(),
            username: username.map(ToString::to_string),
            hostname: hostname.map(ToString::to_string),
        },
    )(input)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("parsing failed: {input}")]
    Parse { input: String, nom: String },
    #[error("invalid utf-8 encoding")]
    InvalidUtf8(#[from] FromUtf8Error),
}

#[cfg(test)]
mod test {
    use nom::combinator::all_consuming;

    use crate::{Source, User};

    #[test]
    fn user() {
        let tests = [
            "dan!d@localhost",
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
                ":atw.hu.quakenet.org ",
                Source::Server("atw.hu.quakenet.org".into()),
            ),
            (":*.freenode.net ", Source::Server("*.freenode.net".into())),
        ];

        for (test, expected) in tests {
            let (_, source) = super::source(test).unwrap();
            assert_eq!(source, expected);
        }
    }

    #[test]
    fn message() {
        let tests = [
            ":irc.example.com CAP LS * :multi-prefix extended-join sasl\r\n",
            "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!\r\n",
            "CAP REQ :sasl\r\n",
            "@tag=as\\\\\\:\\sdf\\z\\ UNKNOWN\r\n",
            "@+1.1.1.1/wi2-asef-1=as\\\\\\:\\sdf\\z\\ UNKNOWN\r\n",
            ":test!test@5555:5555:0:55:5555:5555:5555:5555 396 test user/test :is now your visible host\r\n",
            ":atw.hu.quakenet.org 001 test :Welcome to the QuakeNet IRC Network, test\r\n",
        ];

        for test in tests {
            let message = super::message(test).unwrap();
            println!("{message:?}");
        }
    }
}
