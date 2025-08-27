use std::hash::Hash;
use std::cmp::Ordering;

use std::str::FromStr;

use irc::proto::parse::{Error as ParseError, tagstr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub enum NetworkState {
    Connected,
    Connecting,
    #[default]
    Disconnected,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unable to parse bouncer network: {0}")]
    Parse(#[from] ParseError),
    #[error("Bouncer network missing field: {0}")]
    MissingField(&'static str),
    #[error("Invalid network state: {0}")]
    InvalidState(String),
}

impl FromStr for NetworkState {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "connected" => Ok(Self::Connected),
            "connecting" => Ok(Self::Connecting),
            "disconnected" => Ok(Self::Disconnected),
            s => Err(Error::InvalidState(s.to_owned())),
        }
    }
}


// https://codeberg.org/emersion/soju/src/branch/master/doc/ext/bouncer-networks.md
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BouncerNetwork {
    pub id: String,
    pub name: String,
}

// for ordering, we try to order lexiographically by name, and then check ID
// So ID equality must imply name equality
impl Ord for BouncerNetwork {
    fn cmp(&self, other: &Self) -> Ordering {
        // case sensitive first, then insensitive, then ID
        self.name.to_lowercase().cmp(&other.name.to_lowercase())
            .then_with(|| self.name.cmp(&other.name))
            .then_with(|| self.id.cmp(&other.id))
    }
}

impl PartialOrd for BouncerNetwork {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for BouncerNetwork {
    // we only care about the id for equality
    // It's up to the caller to ensure that the server matches
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for BouncerNetwork {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for BouncerNetwork {}

impl BouncerNetwork {
    pub fn parse(id: &str, s: &str) -> Result<Self, Error> {
        let mut parameter_map = tagstr(s)?;

        Ok(BouncerNetwork {
            id: id.to_owned(),
            name: parameter_map
                .remove("name")
                .ok_or(Error::MissingField("name"))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_network() {
        assert_eq!(
            BouncerNetwork {
                id: 44.to_string(),
                name: "OFTC".to_owned(),
            },
            BouncerNetwork::parse(
                "44",
                "name=OFTC;host=irc.oftc.net;state=connecting"
            )
            .unwrap()
        );
    }
}
