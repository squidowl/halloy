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

    fn decode<'a>(args: impl Iterator<Item = &'a str>) -> Option<Self> {
        let mut args: Vec<&str> = args.collect();
        args.reverse();

        // if token doesn't exist
        let mut token = None;
        let mut size = args[0];
        let mut port = args[1];
        let mut host = args[2];
        let mut filename = args.iter().skip(3);

        // If token exists, port == 0
        // args[1] == port, if token doesn't exists.
        // args[2] == port, if token exists.
        if args[1].parse::<u16>() == Ok(0) || args[2].parse::<u16>() == Ok(0) {
            token = Some(args[0].parse::<NonZeroU16>().ok()?);
            size = args[1];
            port = args[2];
            host = args[3];
            filename = args.iter().skip(4);
        }

        // Parse values
        let host = decode_host(host)?;
        let port = NonZeroU16::new(port.parse().ok()?);
        let size = size.parse().ok()?;
        let filename = filename.rev().copied().collect::<Vec<_>>().join(" ");

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
