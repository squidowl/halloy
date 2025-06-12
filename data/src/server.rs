use std::collections::BTreeMap;
use std::{fmt, str};
use std::sync::Arc;

use std::ops::Deref;
use futures::channel::mpsc::Sender;
use irc::proto;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::process::Command;

use crate::config;
use crate::config::Error;
use crate::config::server::Sasl;

pub type Handle = Sender<proto::Message>;

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct Server(Arc<str>);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ServerConfig(Arc<config::Server>);

impl AsRef<config::Server> for ServerConfig {
    fn as_ref(&self) -> &config::Server {
        &self.0
    }
}
impl Deref for ServerConfig {
    type Target = config::Server;

    fn deref(&self) -> &config::Server {
        &self.0
    }
}
impl From<config::Server> for ServerConfig {
    fn from(inner: config::Server) -> Self {
        Self(Arc::new(inner))
    }
}

impl From<&str> for Server {
    fn from(value: &str) -> Self {
        Server(Arc::from(value))
    }
}

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
    pub config: ServerConfig,
}

impl<'a> From<(&'a Server, &'a ServerConfig)> for Entry {
    fn from((server, config): (&'a Server, &'a ServerConfig)) -> Self {
        Self {
            server: server.clone(),
            config: config.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Map(BTreeMap<Server, ServerConfig>);

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
    pub fn insert(&mut self, name: Server, server: ServerConfig) {
        self.0.insert(name, server);
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

    pub async fn read_passwords(&mut self) -> Result<(), Error> {
        for (_, config_arc) in self.0.iter_mut() {
            // Here the config is unlikely to be shared anywhere,
            // as we are initializing the config.
            let config = Arc::make_mut(&mut config_arc.0);
            if let Some(pass_file) = &config.password_file {
                if config.password.is_some()
                    || config.password_command.is_some()
                {
                    return Err(Error::DuplicatePassword);
                }
                let mut pass = fs::read_to_string(pass_file).await?;
                if config.password_file_first_line_only {
                    pass = pass
                        .lines()
                        .next()
                        .map(String::from)
                        .unwrap_or_default();
                }
                config.password = Some(pass);
            }
            if let Some(pass_command) = &config.password_command {
                if config.password.is_some() {
                    return Err(Error::DuplicatePassword);
                }
                config.password = Some(read_from_command(pass_command).await?);
            }
            if let Some(nick_pass_file) = &config.nick_password_file {
                if config.nick_password.is_some()
                    || config.nick_password_command.is_some()
                {
                    return Err(Error::DuplicateNickPassword);
                }
                let mut nick_pass = fs::read_to_string(nick_pass_file).await?;
                if config.nick_password_file_first_line_only {
                    nick_pass = nick_pass
                        .lines()
                        .next()
                        .map(String::from)
                        .unwrap_or_default();
                }
                config.nick_password = Some(nick_pass);
            }
            if let Some(nick_pass_command) = &config.nick_password_command {
                if config.nick_password.is_some() {
                    return Err(Error::DuplicateNickPassword);
                }
                config.nick_password =
                    Some(read_from_command(nick_pass_command).await?);
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
                        password_file_first_line_only,
                        password_command: None,
                        ..
                    } => {
                        let mut pass = fs::read_to_string(pass_file).await?;
                        if password_file_first_line_only
                            .is_none_or(|first_line_only| first_line_only)
                        {
                            pass = pass
                                .lines()
                                .next()
                                .map(String::from)
                                .unwrap_or_default();
                        }

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
}
