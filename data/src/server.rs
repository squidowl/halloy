use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Server {
    name: String,
    hostname: String,
}

impl Server {
    pub fn new(name: impl ToString, hostname: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            hostname: hostname.to_string(),
        }
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config(irc::client::data::Config);

impl From<irc::client::data::Config> for Config {
    fn from(config: irc::client::data::Config) -> Self {
        Self(config)
    }
}

impl Deref for Config {
    type Target = irc::client::data::Config;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
