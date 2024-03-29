use std::{
    net::{IpAddr, Ipv4Addr},
    num::NonZeroU16,
};

use irc::proto::{self, command};

pub fn decode(content: &str) -> Option<Command> {
    let payload = ctcp_payload(content)?;

    let mut args = payload.split_ascii_whitespace();

    if args.next()? != "DCC" {
        return None;
    }

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

    fn decode<'a>(mut args: impl Iterator<Item = &'a str>) -> Option<Self> {
        let filename = args.next()?.to_string();
        let host = args.next().and_then(decode_host)?;
        let port = NonZeroU16::new(args.next()?.parse().ok()?);
        let size = args.next()?.parse().ok()?;
        let token = args.next();

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

    pub fn encode(self, target: impl ToString) -> proto::Message {
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

fn ctcp_payload(content: &str) -> Option<&str> {
    if content.starts_with('\u{1}') && content.ends_with('\u{1}') && content.len() > 2 {
        Some(&content[1..content.len() - 1])
    } else {
        None
    }
}
