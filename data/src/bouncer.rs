use std::collections::HashMap;

use std::str::FromStr;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum NetworkState {
    Connected,
    Connecting,
    #[default]
    Disconnected,
}

impl FromStr for NetworkState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "connected" => Ok(Self::Connected),
            "connecting" => Ok(Self::Connecting),
            "disconnected" => Ok(Self::Disconnected),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct BouncerNetwork {
    pub id: String,
    pub name: String,
    pub host: String,
    pub state: NetworkState,
    pub port: Option<u16>,
    pub use_tls: Option<bool>,
    pub pass: Option<String>,
    pub nickname: Option<String>,
    pub realname: Option<String>,
    pub error: Option<String>,
}

impl BouncerNetwork {
    pub fn parse(id: &str, s: &str) -> Option<Self> {
        // TODO these need to be tag-decoded, but this functionality is locked in the message
        // parsing module
        let parameter_map: HashMap<_, _> =
            s.split(';').filter_map(|k| k.split_once('=')).collect();

        Some(BouncerNetwork {
            id: id.to_owned(),
            name: parameter_map.get("name")?.to_string(),
            host: parameter_map.get("host")?.to_string(),
            port: parameter_map.get("port").and_then(|s| s.parse().ok()),
            nickname: parameter_map.get("nickname").map(|s| s.to_string()),
            realname: parameter_map.get("realname").map(|s| s.to_string()),
            pass: parameter_map.get("pass").map(|s| s.to_string()),
            state: parameter_map.get("state")?.parse().ok()?,
            use_tls: match parameter_map.get("port").copied() {
                Some("1") => Some(true),
                Some("0") => Some(false),
                _ => None,
            },
            error: parameter_map.get("error").map(|s| s.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_network() {
        assert_eq!(BouncerNetwork {
            id: 44.to_string(),
            name: "OFTC".to_owned(),
            host: "irc.oftc.net".to_owned(),
            state: NetworkState::Connecting,
            ..Default::default()
        }, BouncerNetwork::parse("44", "name=OFTC;host=irc.oftc.net;state=connecting").unwrap());
    }
}
