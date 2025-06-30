use std::fmt::Write;

use itertools::Itertools;

use crate::{Command, Message, Tags};

/// Most IRC servers limit messages to 512 bytes in length, including the trailing CR-LF characters.
pub const BYTE_LIMIT: usize = 512;

pub fn message(message: Message) -> String {
    let mut output = String::with_capacity(BYTE_LIMIT);

    let tags = tags(message.tags);

    if !tags.is_empty() {
        let _ = write!(&mut output, "@{tags} ");
    }

    if let Command::Raw(raw) = &message.command {
        let _ = write!(&mut output, "{raw}");
    } else {
        let command = message.command.command();
        let params = parameters(message.command.parameters());

        let _ = write!(&mut output, "{command} {params}");
    }

    let _ = write!(&mut output, "\r\n");

    output
}

fn tags(tags: Tags) -> String {
    tags.into_iter().map(tag).join(";")
}

fn tag((key, value): (String, String)) -> String {
    if value.is_empty() {
        return key;
    }

    let mappings = [
        ('\\', r"\\"),
        (';', r"\:"),
        (' ', r"\s"),
        ('\r', r"\r"),
        ('\n', r"\n"),
    ];

    let escaped = mappings
        .into_iter()
        .fold(value, |value, (from, to)| value.replace(from, to));

    format!("{key}={escaped}")
}

fn parameters(parameters: Vec<String>) -> String {
    let params_len = parameters.len();
    parameters
        .into_iter()
        .enumerate()
        .map(|(index, param)| {
            if index == params_len - 1 {
                trailing(param)
            } else {
                param
            }
        })
        .join(" ")
}

fn trailing(parameter: String) -> String {
    if parameter.contains(' ')
        || parameter.is_empty()
        || parameter.starts_with(':')
    {
        format!(":{parameter}")
    } else {
        parameter
    }
}

#[cfg(test)]
mod test {
    use crate::{command, format};

    #[test]
    fn commands() {
        let tests = [
            command!("CAP", "LS", "302"),
            command!("privmsg", "#a", "nospace"),
            command!("privmsg", "b", "spa ces"),
            command!("quit", "nocolon"),
            command!("quit", ":startscolon"),
            command!("quit", "not:starting"),
            command!("quit", "not:starting space"),
            command!("notice", ""),
            command!("notice", " "),
            command!("USER", "test", "test"),
        ];
        let expected = [
            "CAP LS 302\r\n",
            "PRIVMSG #a nospace\r\n",
            "PRIVMSG b :spa ces\r\n",
            "QUIT nocolon\r\n",
            "QUIT ::startscolon\r\n",
            "QUIT not:starting\r\n",
            "QUIT :not:starting space\r\n",
            "NOTICE :\r\n",
            "NOTICE : \r\n",
            "USER test 0 * test\r\n",
        ];

        for (test, expected) in tests.into_iter().zip(expected) {
            let formatted = format::message(test);
            assert_eq!(formatted, expected);
        }
    }

    #[test]
    fn tags() {
        let test = tags![
            "tag" => "as\\; \r\n",
            "id" => "234AB",
            "test" => "",
        ];
        let expected = r"id=234AB;tag=as\\\:\s\r\n;test";

        let tags = super::tags(test);
        assert_eq!(tags, expected);
    }
}
