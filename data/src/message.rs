use irc::proto::{Command, Response};

pub enum Source {
    Server,
    Channel(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    Sent {
        nickname: String,
        message: irc::proto::Message,
    },
    Received(irc::proto::Message),
}

impl Message {
    fn inner(&self) -> &irc::proto::Message {
        match self {
            Message::Sent { message, .. } => message,
            Message::Received(message) => message,
        }
    }

    pub fn text(&self) -> Option<&str> {
        text(self.inner())
    }

    pub fn source(&self, channels: &[String]) -> Option<Source> {
        if self.is_server_message(channels) {
            Some(Source::Server)
        } else {
            channels.iter().find_map(|channel| {
                is_for_channel(self.inner(), channel).then(|| Source::Channel(channel.to_string()))
            })
        }
    }

    pub fn is_for_channel(&self, channel: &str) -> bool {
        is_for_channel(self.inner(), channel)
    }

    pub fn is_server_message(&self, channels: &[String]) -> bool {
        match self {
            Message::Sent { .. } => false,
            Message::Received(message) => is_for_server(message, channels),
        }
    }

    pub fn nickname(&self) -> Option<&str> {
        match &self {
            Message::Sent { nickname, .. } => Some(nickname.as_str()),
            Message::Received(message) => message.prefix.as_ref().and_then(|prefix| match prefix {
                irc::proto::Prefix::ServerName(_) => None,
                irc::proto::Prefix::Nickname(nickname, _, _) => Some(nickname.as_str()),
            }),
        }
    }
}

fn text(message: &irc::proto::Message) -> Option<&str> {
    match &message.command {
        Command::PRIVMSG(_, text) | Command::NOTICE(_, text) => Some(text),
        Command::Response(_, responses) => responses.last().map(String::as_str),
        _ => None,
    }
}

fn is_for_channel(message: &irc::proto::Message, channel: &str) -> bool {
    match &message.command {
        Command::PRIVMSG(target, _) | Command::NOTICE(target, _) | Command::TOPIC(target, _) => {
            target == channel
        }
        _ => false,
    }
}

fn is_for_server(message: &irc::proto::Message, channels: &[String]) -> bool {
    match &message.command {
        Command::NICK(_) => true,
        Command::NOTICE(target, _) => !channels.contains(target),
        Command::Response(response, _) => match response {
            Response::RPL_WELCOME => true,
            Response::RPL_MOTDSTART => true,
            Response::RPL_MOTD => true,
            Response::RPL_ENDOFMOTD => true,
            _ => false, // TODO: Are there others we want to show??
        },
        _ => false,
    }
}
