// Reference: https://rawgit.com/DanielOaks/irc-rfcs/master/dist/draft-oakley-irc-ctcp-latest.html

#[derive(Debug)]
pub enum Command {
    Action,
    ClientInfo,
    DCC,
    Ping,
    Source,
    Version,
    Unknown(String),
}

#[derive(Debug)]
pub struct Query<'a> {
    pub command: Command,
    pub params: Option<&'a str>,
}

pub fn is_query(text: &str) -> bool {
    text.starts_with('\u{1}')
}

pub fn parse_query(text: &str) -> Option<Query> {
    let query = text
        .strip_suffix('\u{1}')
        .unwrap_or(text)
        .strip_prefix('\u{1}')?;

    let (command, params) = if let Some((command, params)) = query.split_once(char::is_whitespace) {
        (command.to_uppercase(), Some(params))
    } else {
        (query.to_uppercase(), None)
    };

    let command = match command.as_ref() {
        "ACTION" => Command::Action,
        "CLIENTINFO" => Command::ClientInfo,
        "DCC" => Command::DCC,
        "PING" => Command::Ping,
        "SOURCE" => Command::Source,
        "VERSION" => Command::Version,
        _ => Command::Unknown(command),
    };

    Some(Query { command, params })
}

pub fn format(command: &Command, params: Option<&str>) -> String {
    let command = match command {
        Command::Action => "ACTION",
        Command::ClientInfo => "CLIENTINFO",
        Command::DCC => "DCC",
        Command::Ping => "PING",
        Command::Source => "SOURCE",
        Command::Version => "VERSION",
        Command::Unknown(command) => command.as_ref(),
    };

    if let Some(params) = params {
        format!("\u{1}{command} {params}\u{1}")
    } else {
        format!("\u{1}{command}\u{1}")
    }
}
