use std::sync::Arc;
use std::{cmp, fmt, str};

use futures::channel::mpsc::Sender;
use futures::{StreamExt, TryStreamExt, stream};
use indexmap::IndexMap;
use irc::proto;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::process::Command;

use crate::config;
use crate::config::Error;
use crate::config::server::Sasl;

pub type Handle = Sender<proto::Message>;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Server(Arc<str>);

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

// Use case-insensitive comparison first, falling back to case-sensitive
// only when server names are equal (in a case-insensitive context).
impl Ord for Server {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let case_insensitive_ordering =
            self.0.to_lowercase().cmp(&other.0.to_lowercase());

        match case_insensitive_ordering {
            cmp::Ordering::Equal => self.0.cmp(&other.0),
            _ => case_insensitive_ordering,
        }
    }
}

impl PartialOrd for Server {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub server: Server,
    pub config: Arc<config::Server>,
}

impl<'a> From<(&'a Server, &'a Arc<config::Server>)> for Entry {
    fn from((server, config): (&'a Server, &'a Arc<config::Server>)) -> Self {
        Self {
            server: server.clone(),
            config: config.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Map(IndexMap<Server, Arc<config::Server>>);

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
    pub async fn new(
        iter: impl IntoIterator<Item = (Server, config::Server)>,
    ) -> Result<Self, Error> {
        let inner = stream::iter(iter)
            .then(|(server, mut config)| async move {
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
                    config.password =
                        Some(read_from_command(pass_command).await?);
                }
                if let Some(nick_pass_file) = &config.nick_password_file {
                    if config.nick_password.is_some()
                        || config.nick_password_command.is_some()
                    {
                        return Err(Error::DuplicateNickPassword);
                    }
                    let mut nick_pass =
                        fs::read_to_string(nick_pass_file).await?;
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
                            let mut pass =
                                fs::read_to_string(pass_file).await?;
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

                Ok((server, Arc::new(config)))
            })
            .try_collect()
            .await?;

        Ok(Self(inner))
    }

    pub fn insert(&mut self, server: Server, config: config::Server) {
        self.0.insert(server, Arc::new(config));
    }

    pub fn remove(&mut self, server: &Server) {
        self.0.shift_remove(server);
    }

    pub fn contains(&self, server: &Server) -> bool {
        self.0.contains_key(server)
    }

    pub fn get(&self, server: &Server) -> Option<Arc<config::Server>> {
        self.0.get(server).cloned()
    }

    pub fn keys(&self) -> impl Iterator<Item = &Server> {
        self.0.keys()
    }

    pub fn entries(&self) -> impl Iterator<Item = Entry> + '_ {
        self.0.iter().map(Entry::from)
    }
}
