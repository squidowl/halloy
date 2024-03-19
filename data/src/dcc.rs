use std::{
    net::{IpAddr, Ipv4Addr},
    num::NonZeroU16,
};

use irc::proto;

pub fn decode(message: &proto::Message) -> Option<Command> {
    let payload = ctcp_payload(&message.command)?;

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

#[derive(Debug)]
pub enum Command {
    Send(Send),
}

#[derive(Debug)]
pub struct Send {
    pub secure: bool,
    pub filename: String,
    pub direction: Direction,
}

impl Send {
    fn decode<'a>(secure: bool, mut args: impl Iterator<Item = &'a str>) -> Option<Self> {
        let filename = args.next()?.to_string();
        let direction = Direction::decode(&mut args)?;

        Some(Send {
            secure,
            filename,
            direction,
        })
    }
}

#[derive(Debug)]
pub enum Direction {
    Reverse {
        token: String,
        size: u64,
    },
    Direct {
        host: IpAddr,
        port: NonZeroU16,
        size: u64,
    },
}

impl Direction {
    fn decode<'a>(mut args: impl Iterator<Item = &'a str>) -> Option<Self> {
        let first = args.next()?;

        if first == "0" {
            // TODO: Hexchat seems to specify token before file size??
            // Validate how other clients send this since this may be wrong
            // in general.
            let token = args.next()?.to_string();
            let size = args.next()?.parse().ok()?;

            Some(Direction::Reverse { token, size })
        } else {
            let host = decode_host(first)?;
            let port = args.next().and_then(decode_port)?;
            let size = args.next()?.parse().ok()?;

            Some(Direction::Direct { host, port, size })
        }
    }
}

fn decode_host(host: &str) -> Option<IpAddr> {
    match host.parse::<u32>() {
        Ok(n) => Some(IpAddr::V4(Ipv4Addr::from(n.to_be_bytes()))),
        Err(_) => host.parse().ok(),
    }
}

fn decode_port(port: &str) -> Option<NonZeroU16> {
    NonZeroU16::new(port.parse().ok()?)
}

fn ctcp_payload(command: &proto::Command) -> Option<&str> {
    // TODO: Is NOTICE ever used for DCC? Guess we don't care since it shouldn't
    // impact control flow
    let (proto::Command::PRIVMSG(_, content) | proto::Command::NOTICE(_, content)) = command else {
        return None;
    };

    if content.starts_with('\u{1}') && content.ends_with('\u{1}') && content.len() > 2 {
        Some(&content[1..content.len() - 1])
    } else {
        None
    }
}
