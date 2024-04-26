use std::collections::BTreeMap;
use std::fmt;
use std::fs;

use futures::channel::mpsc::Sender;
use irc::proto;
use serde::{Deserialize, Serialize};

use crate::config;
use crate::config::server::Sasl;
use crate::config::Error;

pub type Handle = Sender<proto::Message>;

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

    pub fn contains(&self, server: &Server) -> bool {
        self.0.contains_key(server)
    }

    pub fn keys(&self) -> impl Iterator<Item = &Server> {
        self.0.keys()
    }

    pub fn entries(&self) -> impl Iterator<Item = Entry> + '_ {
        self.0.iter().map(Entry::from)
    }

    pub fn read_password_files(&mut self) -> Result<(), Error> {
        for (_, config) in self.0.iter_mut() {
            if let Some(pass_file) = &config.password_file {
                if config.password.is_some() {
                    return Err(Error::Parse(
                        "Only one of password and password_file can be set.".to_string(),
                    ));
                }
                let pass = fs::read_to_string(pass_file)?;
                config.password = Some(pass);
            }
            if let Some(nick_pass_file) = &config.nick_password_file {
                if config.nick_password.is_some() {
                    return Err(Error::Parse(
                        "Only one of nick_password and nick_password_file can be set.".to_string(),
                    ));
                }
                let nick_pass = fs::read_to_string(nick_pass_file)?;
                config.nick_password = Some(nick_pass);
            }
            if let Some(sasl) = &mut config.sasl {
                match sasl {
                    Sasl::Plain {
                        password: Some(_),
                        password_file: Some(_),
                        ..
                    } => {
                        return Err(Error::Parse("Exactly one of sasl.plain.password or sasl.plain.password_file must be set.".to_string()));
                    }
                    Sasl::Plain {
                        password: password @ None,
                        password_file: Some(pass_file),
                        ..
                    } => {
                        let pass = fs::read_to_string(pass_file)?;
                        *password = Some(pass);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
