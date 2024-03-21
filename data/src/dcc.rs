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
        "send" => Send::decode(false, args).map(Command::Send),
        "ssend" => Send::decode(true, args).map(Command::Send),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Send(Send),
}

#[derive(Debug, Clone)]
pub enum Send {
    Reverse {
        secure: bool,
        filename: String,
        size: u64,
        token: String,
    },
    Direct {
        secure: bool,
        filename: String,
        host: IpAddr,
        port: NonZeroU16,
        size: u64,
        token: Option<String>,
    },
}

impl Send {
    pub fn secure(&self) -> bool {
        match self {
            Send::Reverse { secure, .. } => *secure,
            Send::Direct { secure, .. } => *secure,
        }
    }

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
            Send::Direct { token, .. } => token.as_deref(),
        }
    }

    fn decode<'a>(secure: bool, mut args: impl Iterator<Item = &'a str>) -> Option<Self> {
        let filename = args.next()?.to_string();
        let port_or_host = args.next()?;

        if port_or_host == "0" {
            let size = args.next()?.parse().ok()?;
            let token = args.next()?.to_string();

            Some(Self::Reverse {
                secure,
                filename,
                size,
                token,
            })
        } else {
            let host = decode_host(port_or_host)?;
            let port = args.next().and_then(decode_port)?;
            let size = args.next()?.parse().ok()?;
            let token = args.next().map(String::from);

            Some(Self::Direct {
                secure,
                filename,
                host,
                port,
                size,
                token,
            })
        }
    }

    pub fn encode(self, target: impl ToString) -> proto::Message {
        match self {
            Self::Reverse {
                secure,
                filename,
                token,
                size,
            } => {
                let kind = if secure { "SSEND" } else { "SEND" };

                command!(
                    "PRIVMSG",
                    target.to_string(),
                    format!("\u{0}DCC {kind} {filename} 0 {size} {token}\u{0}")
                )
            }
            Self::Direct {
                secure,
                filename,
                host,
                port,
                size,
                token,
            } => {
                let kind = if secure { "SSEND" } else { "SEND" };
                let host = match host {
                    IpAddr::V4(v4) => u32::from(v4).to_string(),
                    IpAddr::V6(v6) => v6.to_string(),
                };
                let token = token.map(|t| format!(" {t}")).unwrap_or_default();

                command!(
                    "PRIVMSG",
                    target.to_string(),
                    format!("\u{0}DCC {kind} {filename} {host} {port} {size}{token}\u{0}")
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

fn decode_port(port: &str) -> Option<NonZeroU16> {
    NonZeroU16::new(port.parse().ok()?)
}

fn ctcp_payload(content: &str) -> Option<&str> {
    if content.starts_with('\u{1}') && content.ends_with('\u{1}') && content.len() > 2 {
        Some(&content[1..content.len() - 1])
    } else {
        None
    }
}
