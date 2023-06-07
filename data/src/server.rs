use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Server(String);

impl From<String> for Server {
    fn from(server: String) -> Self {
        Self(server)
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
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
