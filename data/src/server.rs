use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Server(String);

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for Server {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
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

#[derive(Debug, Clone)]
pub struct Entry {
    pub server: Server,
    pub config: Config,
}

impl<'a> From<(&'a Server, &'a Config)> for Entry {
    fn from((server, config): (&'a Server, &'a Config)) -> Self {
        Self {
            server: server.clone(),
            config: config.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Map(BTreeMap<Server, Config>);

impl Map {
    pub fn remove(&mut self, server: &Server) {
        self.0.remove(server);
    }

    pub fn entries(&self) -> impl Iterator<Item = Entry> + '_ {
        self.0.iter().map(Entry::from)
    }
}
