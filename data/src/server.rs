use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::config;

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

#[derive(Debug, Clone)]
pub struct Entry {
    pub server: Server,
    pub config: config::Server,
}

impl<'a> From<(&'a Server, &'a config::Server)> for Entry {
    fn from((server, config): (&'a Server, &'a config::Server)) -> Self {
        Self {
            server: server.clone(),
            config: config.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Map(BTreeMap<Server, config::Server>);

impl Map {
    pub fn remove(&mut self, server: &Server) {
        self.0.remove(server);
    }

    pub fn entries(&self) -> impl Iterator<Item = Entry> + '_ {
        self.0.iter().map(Entry::from)
    }
}
