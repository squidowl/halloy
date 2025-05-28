use std::net::{IpAddr, Ipv4Addr};
use std::num::NonZeroU16;

use irc::proto;
use itertools::Itertools;

use crate::ctcp;

pub fn decode(content: &str) -> Option<Command> {
    let query = ctcp::parse_query(content)?;

    if !matches!(query.command, ctcp::Command::DCC) {
        return None;
    }

    let mut args = query.params.map(|params| params.split_whitespace())?;

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

#[derive(Debug, Clone, PartialEq, Eq)]
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
        let args = args.collect::<Vec<_>>();

        if args.len() < 4 {
            return None;
        }

        // Host will always be 3rd or 4th arg in reverse order
        // The last arg to successfully decode as host will be host
        let host_pos = args.len()
            - 1
            - args
                .iter()
                .rev()
                .take(if args.len() > 4 { 4 } else { 3 })
                .enumerate()
                .filter_map(|(i, arg)| decode_host(arg).map(|_| i))
                .next_back()?;

        let filename = args
            .iter()
            .take(host_pos)
            .join(" ")
            .trim_matches('\"')
            .to_string();

        let mut remaining_args = args.into_iter().skip(host_pos);

        let host = remaining_args.next().and_then(decode_host)?;
        let port = NonZeroU16::new(remaining_args.next()?.parse().ok()?);
        let size = remaining_args.next()?.parse().ok()?;
        let token = remaining_args.next();

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
                let port = port.map_or(0, NonZeroU16::get);

                ctcp::query_message(
                    &ctcp::Command::DCC,
                    target.to_string(),
                    Some(format!(
                        "SEND {filename} {host} {port} {size} {token}"
                    )),
                )
            }
            Self::Direct {
                filename,
                host,
                port,
                size,
            } => {
                let host = encode_host(host);

                ctcp::query_message(
                    &ctcp::Command::DCC,
                    target.to_string(),
                    Some(format!("SEND {filename} {host} {port} {size}")),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_decode() {
        let args = "my_file_name 1402301083 12350 1453953495";
        let send = Send::decode(args.split_whitespace());
        assert_eq!(
            send,
            Some(Send::Direct {
                filename: "my_file_name".to_string(),
                host: IpAddr::V4(Ipv4Addr::from(1402301083)),
                port: NonZeroU16::new(12350).unwrap(),
                size: 1453953495
            })
        );
    }

    #[test]
    fn send_decode_whitespace() {
        let args = "my file name 1402301083 12350 1453953495";
        let send = Send::decode(args.split_whitespace());
        assert_eq!(
            send,
            Some(Send::Direct {
                filename: "my file name".to_string(),
                host: IpAddr::V4(Ipv4Addr::from(1402301083)),
                port: NonZeroU16::new(12350).unwrap(),
                size: 1453953495
            })
        );
    }

    #[test]
    fn send_decode_quotation_marks() {
        let args = "\"my file name\" 1402301083 12350 1453953495";
        let send = Send::decode(args.split_whitespace());
        assert_eq!(
            send,
            Some(Send::Direct {
                filename: "my file name".to_string(),
                host: IpAddr::V4(Ipv4Addr::from(1402301083)),
                port: NonZeroU16::new(12350).unwrap(),
                size: 1453953495
            })
        );
    }

    #[test]
    fn send_decode_token() {
        let args = "\"my file name\" 1402301083 12345 1453953495 token";
        let send = Send::decode(args.split_whitespace());
        assert_eq!(
            send,
            Some(Send::Reverse {
                filename: "my file name".to_string(),
                host: IpAddr::V4(Ipv4Addr::from(1402301083)),
                port: NonZeroU16::new(12345),
                size: 1453953495,
                token: "token".to_string()
            })
        );
    }

    #[test]
    fn send_decode_port_zero() {
        // Non-zero host is required when token is missing
        let args = "\"my file name\" 1402301083 0 1453953495";
        let send = Send::decode(args.split_whitespace());
        assert_eq!(send, None);

        // Works because token is provided
        let args = "\"my file name\" 1402301083 0 1453953495 token";
        let send = Send::decode(args.split_whitespace());
        assert_eq!(
            send,
            Some(Send::Reverse {
                filename: "my file name".to_string(),
                host: IpAddr::V4(Ipv4Addr::from(1402301083)),
                port: None,
                size: 1453953495,
                token: "token".to_string()
            })
        );
    }

    #[test]
    fn send_decode_numeric_filename() {
        // Succeeds because only 4 args so we know to only
        // check up to last 3 for host
        let args = "2 1402301083 12345 1453953495";
        let send = Send::decode(args.split_whitespace());
        assert_eq!(
            send,
            Some(Send::Direct {
                filename: "2".to_string(),
                host: IpAddr::V4(Ipv4Addr::from(1402301083)),
                port: NonZeroU16::new(12345).unwrap(),
                size: 1453953495,
            })
        );

        // Succeeds because >= 5 args with token so last 4
        // never overlap w/ filename
        let args = "filename 2 1402301083 12345 1453953495 token";
        let send = Send::decode(args.split_whitespace());
        assert_eq!(
            send,
            Some(Send::Reverse {
                filename: "filename 2".to_string(),
                host: IpAddr::V4(Ipv4Addr::from(1402301083)),
                port: NonZeroU16::new(12345),
                size: 1453953495,
                token: "token".to_string(),
            })
        );

        // Fails because >= 5 args without token AND filename
        // ending in numeric, so 4th arg (part of filename) is parsed
        // as host and 3rd arg cannot be parsed as a u16 port so it
        // returns None
        let args = "filename 2 1402301083 12345 1453953495";
        let send = Send::decode(args.split_whitespace());
        assert_eq!(send, None);
    }
}
