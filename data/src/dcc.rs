use std::{
    net::{IpAddr, Ipv4Addr},
    num::NonZeroU16,
};

use crate::ctcp;
use irc::proto::{self, command};

pub fn decode(content: &str) -> Option<Command> {
    let query = ctcp::parse_query(content)?;

    if query.command != "DCC" {
        return None;
    }

    let mut args = query.params.split_whitespace();

    match args.next()?.to_lowercase().as_str() {
        "send" => Send::decode(args).map(Command::Send),
        cmd => Some(Command::Unsupported(cmd.to_string())),
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Send(Send),
    Unsupported(String),
}

#[derive(Debug, Clone)]
pub enum Send {
    Reverse {
        filename: String,
        host: IpAddr,
        port: Option<NonZeroU16>,
        size: u64,
        token: String,
    },
    Direct {
        filename: String,
        host: IpAddr,
        port: NonZeroU16,
        size: u64,
    },
}

impl Send {
    pub fn filename(&self) -> &str {
        match self {
            Send::Reverse { filename, .. } => filename,
            Send::Direct { filename, .. } => filename,
        }
    }

    pub fn size(&self) -> u64 {
        match self {
            Send::Reverse { size, .. } => *size,
            Send::Direct { size, .. } => *size,
        }
    }

    pub fn token(&self) -> Option<&str> {
        match self {
            Send::Reverse { token, .. } => Some(token),
            Send::Direct { .. } => None,
        }
    }

    fn decode<'a>(
        args: impl std::iter::DoubleEndedIterator<Item = &'a str> + std::clone::Clone,
    ) -> Option<Self> {
        let mut args = args.rev();
        let mut args_token = args.clone();

        // if token doesn't exist
        let mut token = None;
        let mut size = args.next()?;
        let mut port = args.next()?;
        let mut host = args.next()?;
        let mut filename = args;

        // If token exists, port == 0
        // args[1] == port, if token doesn't exists.
        // args[2] == port, if token exists.
        if size.parse::<u16>() == Ok(0) || port.parse::<u16>() == Ok(0) {
            token = Some(args_token.next()?.parse::<NonZeroU16>().ok()?);
            size = args_token.next()?;
            port = args_token.next()?;
            host = args_token.next()?;
            filename = args_token;
        }

        // Parse values
        let host = decode_host(host)?;
        let port = NonZeroU16::new(port.parse().ok()?);
        let size = size.parse().ok()?;
        let filename = filename
            .rev()
            .map(|s| s.trim_matches('\"'))
            .collect::<Vec<_>>()
            .join(" ");

        match (port, token) {
            (_, Some(token)) => Some(Self::Reverse {
                filename,
                host,
                port,
                size,
                token: token.to_string(),
            }),
            (Some(port), None) => Some(Self::Direct {
                filename,
                host,
                port,
                size,
            }),
            _ => None,
        }
    }

    pub fn encode(self, target: &dyn ToString) -> proto::Message {
        match self {
            Self::Reverse {
                filename,
                host,
                port,
                size,
                token,
            } => {
                let host = encode_host(host);
                let port = port.map(NonZeroU16::get).unwrap_or(0);

                command!(
                    "PRIVMSG",
                    target.to_string(),
                    format!("\u{1}DCC SEND {filename} {host} {port} {size} {token}\u{1}")
                )
            }
            Self::Direct {
                filename,
                host,
                port,
                size,
            } => {
                let host = encode_host(host);

                command!(
                    "PRIVMSG",
                    target.to_string(),
                    format!("\u{1}DCC SEND {filename} {host} {port} {size}\u{1}")
                )
            }
        }
    }
}

fn decode_host(host: &str) -> Option<IpAddr> {
    match host.parse::<u32>() {
        Ok(n) => Some(IpAddr::V4(Ipv4Addr::from(n))),
        Err(_) => host.parse().ok(),
    }
}

fn encode_host(host: IpAddr) -> String {
    match host {
        IpAddr::V4(v4) => u32::from(v4).to_string(),
        IpAddr::V6(v6) => v6.to_string(),
    }
}
