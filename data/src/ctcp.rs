use std::fmt;

use irc::proto;

// Reference: https://rawgit.com/DanielOaks/irc-rfcs/master/dist/draft-oakley-irc-ctcp-latest.html

#[derive(Debug, Clone)]
pub enum Command {
    Action,
    ClientInfo,
    DCC,
    Ping,
    Source,
    Version,
    Time,
    Unknown(String),
}

impl From<&str> for Command {
    fn from(command: &str) -> Self {
        match command.to_uppercase().as_ref() {
            "ACTION" => Command::Action,
            "CLIENTINFO" => Command::ClientInfo,
            "DCC" => Command::DCC,
            "PING" => Command::Ping,
            "SOURCE" => Command::Source,
            "VERSION" => Command::Version,
            "TIME" => Command::Time,
            _ => Command::Unknown(command.to_string()),
        }
    }
}

impl AsRef<str> for Command {
    fn as_ref(&self) -> &str {
        match self {
            Command::Action => "ACTION",
            Command::ClientInfo => "CLIENTINFO",
            Command::DCC => "DCC",
            Command::Ping => "PING",
            Command::Source => "SOURCE",
            Command::Version => "VERSION",
            Command::Time => "TIME",
            Command::Unknown(command) => command.as_ref(),
        }
    }
}

#[derive(Debug)]
pub struct Query<'a> {
    pub command: Command,
    pub params: Option<&'a str>,
}

pub fn is_query(text: &str) -> bool {
    text.starts_with('\u{1}')
}

pub fn parse_query(text: &str) -> Option<Query<'_>> {
    let query = text
        .strip_suffix('\u{1}')
        .unwrap_or(text)
        .strip_prefix('\u{1}')?;

    let (command, params) = if let Some((command, params)) =
        query.split_once(char::is_whitespace)
    {
        (command.to_uppercase(), Some(params))
    } else {
        (query.to_uppercase(), None)
    };

    let command = Command::from(command.as_str());

    Some(Query { command, params })
}

pub fn format(command: &Command, params: Option<impl fmt::Display>) -> String {
    let command = command.as_ref();

    if let Some(params) = params {
        format!("\u{1}{command} {params}\u{1}")
    } else {
        format!("\u{1}{command}\u{1}")
    }
}

pub fn query_command(
    command: &Command,
    target: String,
    params: Option<impl fmt::Display>,
) -> proto::Command {
    proto::Command::PRIVMSG(target, format(command, params))
}

pub fn query_message(
    command: &Command,
    target: String,
    params: Option<impl fmt::Display>,
) -> proto::Message {
    proto::command!("PRIVMSG", target, format(command, params))
}

pub fn response_message(
    command: &Command,
    target: String,
    params: Option<impl fmt::Display>,
) -> proto::Message {
    proto::command!("NOTICE", target, format(command, params))
}
