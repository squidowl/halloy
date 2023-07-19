use itertools::Itertools;

use crate::Message;

// TODO: Tags
pub fn message(message: Message) -> String {
    let tag = message.command.tag();

    let parameters = message.command.parameters();
    let params_len = parameters.len();
    let params = parameters
        .into_iter()
        .enumerate()
        .map(|(index, param)| {
            if index == params_len - 1 {
                trailing(param)
            } else {
                param
            }
        })
        .join(" ");

    format!("{tag} {params}\r\n")
}

fn trailing(parameter: String) -> String {
    if parameter.contains(' ') || parameter.is_empty() || parameter.starts_with(':') {
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
}
