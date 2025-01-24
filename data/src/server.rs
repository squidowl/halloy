use std::collections::BTreeMap;
use std::sync::Arc;
use std::{fmt, str};
use tokio::fs;
use tokio::process::Command;

use futures::channel::mpsc::Sender;
use irc::proto;
use serde::{Deserialize, Serialize};

use anyhow::{bail, Result};

use crate::bouncer::BouncerNetwork;
use crate::config;
use crate::config::server::Sasl;
use crate::config::Error;

pub type Handle = Sender<proto::Message>;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Server {
    name: Arc<str>,
    #[serde(skip)]
    bouncer_netid: Option<Arc<str>>,
}

impl Server {
    pub fn bouncer_id(&self) -> Option<&str> {
        self.bouncer_netid.as_deref()
    }

    pub fn is_bouncer_network(&self) -> bool {
        self.bouncer_netid.is_some()
    }

    pub fn bouncer_server(&self, network: &BouncerNetwork) -> Self {
        Self {
            name: self.name.clone(),
            bouncer_netid: Some(network.id.as_str().into()),
        }
    }
}

impl From<&str> for Server {
    fn from(value: &str) -> Self {
        Server {
            name: Arc::from(value),
            bouncer_netid: None,
        }
    }
}

// TODO this should be removed
impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(netid) = &self.bouncer_netid {
            write!(f, "{}::{}", self.name, netid)
        } else {
            self.name.fmt(f)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub server: Server,
    pub config: config::Server,
}

impl<'a> From<(&'a Server, &'a MapVal)> for Entry {
    fn from((server, val): (&'a Server, &'a MapVal)) -> Self {
        Self {
            server: server.clone(),
            config: val.config.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(transparent)]
struct MapVal {
    config: config::Server,
    #[serde(skip)]
    bouncer_config: Option<BouncerNetwork>,
}

impl From<config::Server> for MapVal {
    fn from(config: config::Server) -> Self {
        Self {
            config,
            bouncer_config: None,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(transparent)]
pub struct Map(BTreeMap<Server, MapVal>);

async fn read_from_command(pass_command: &str) -> Result<String, Error> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C")
            .arg(pass_command)
            .output()
            .await?
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(pass_command)
            .output()
            .await?
    };
    if output.status.success() {
        // we remove trailing whitespace, which might be present from unix pipelines with a
        // trailing newline
        Ok(str::from_utf8(&output.stdout)?.trim_end().to_string())
    } else {
        Err(Error::ExecutePasswordCommand(String::from_utf8(
            output.stderr,
        )?))
    }
}

impl Map {
    pub fn insert(&mut self, name: Server, server: config::Server) {
        self.0.insert(name, server.into());
    }

    pub fn name_for<'a>(&'a self, server: &Server) -> &'a str {
        // We want to return a lifetime aliasing _only_ that of the configuration mapping, which
        // should live longer than that of the individiual server struct. Unfortunately, this means
        // that we cannot handle the case where the server isn't in the mapping (which shouldn't
        // happen during runtime).
        let Some((key, val)) = self.0.get_key_value(server) else {
            panic!("Server was not in the config mapping");
        };
        if let Some(bouncer_network) = &val.bouncer_config {
            &bouncer_network.name
        } else {
            // key and self are identical
            &key.name
        }
    }

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

    pub fn bouncer_network(&self, server: &Server) -> Option<&BouncerNetwork> {
        self.0.get(server).and_then(|val| val.bouncer_config.as_ref())
    }

    pub async fn read_passwords(&mut self) -> Result<(), Error> {
        for (_, val) in self.0.iter_mut() {
            let config = &mut val.config;
            if let Some(pass_file) = &config.password_file {
                if config.password.is_some() || config.password_command.is_some() {
                    return Err(Error::DuplicatePassword);
                }
                let pass = fs::read_to_string(pass_file).await?;
                config.password = Some(pass);
            }
            if let Some(pass_command) = &config.password_command {
                if config.password.is_some() {
                    return Err(Error::DuplicatePassword);
                }
                config.password = Some(read_from_command(pass_command).await?);
            }
            if let Some(nick_pass_file) = &config.nick_password_file {
                if config.nick_password.is_some() || config.nick_password_command.is_some() {
                    return Err(Error::DuplicateNickPassword);
                }
                let nick_pass = fs::read_to_string(nick_pass_file).await?;
                config.nick_password = Some(nick_pass);
            }
            if let Some(nick_pass_command) = &config.nick_password_command {
                if config.nick_password.is_some() {
                    return Err(Error::DuplicateNickPassword);
                }
                config.nick_password = Some(read_from_command(nick_pass_command).await?);
            }
            if let Some(sasl) = &mut config.sasl {
                match sasl {
                    Sasl::Plain {
                        password: Some(_),
                        password_file: None,
                        password_command: None,
                        ..
                    } => {}
                    Sasl::Plain {
                        password: password @ None,
                        password_file: Some(pass_file),
                        password_command: None,
                        ..
                    } => {
                        let pass = fs::read_to_string(pass_file).await?;
                        *password = Some(pass);
                    }
                    Sasl::Plain {
                        password: password @ None,
                        password_file: None,
                        password_command: Some(pass_command),
                        ..
                    } => {
                        let pass = read_from_command(pass_command).await?;
                        *password = Some(pass);
                    }
                    Sasl::Plain { .. } => {
                        return Err(Error::DuplicateSaslPassword);
                    }
                    Sasl::External { .. } => {
                        // no passwords to read
                    }
                }
            }
        }
        Ok(())
    }

    pub fn insert_bounced_server(&mut self, server: &Server, bouncer_network: BouncerNetwork) -> Result<()> {
        let Some(val) = self.0.get(server) else {
            bail!("Unable to insert bouncer network {:?} because server {:?} does not exist",
                bouncer_network,
                server,
            );
        };
        let bouncer_server = server.bouncer_server(&bouncer_network);
        let bouncer_config = val.config.bouncer_server(&bouncer_network);
        self.0.insert(bouncer_server.clone(), MapVal{ config: bouncer_config, bouncer_config: Some(bouncer_network) });
        Ok(())
}
}
