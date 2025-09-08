use std::cmp::Ordering;
use std::sync::Arc;
use std::{cmp, fmt, str};

use futures::channel::mpsc::Sender;
use indexmap::IndexMap;
use irc::proto;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::process::Command;

use crate::bouncer::BouncerNetwork;
use crate::config::server::Sasl;
use crate::config::sidebar::OrderBy;
use crate::config::{self, Error, sidebar};

pub type Handle = Sender<proto::Message>;

pub type ServerName = Arc<str>;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize)]
pub struct Server {
    pub name: ServerName,
    pub network: Option<Arc<BouncerNetwork>>,
}

impl Server {
    pub fn is_bouncer_network(&self) -> bool {
        self.network.is_some()
    }

    pub fn bouncer_netid(&self) -> Option<&str> {
        self.network.as_ref().map(|network| &*network.id)
    }

    pub fn parent(&self) -> Option<Server> {
        if self.network.is_some() {
            Some(Self {
                network: None,
                ..self.clone()
            })
        } else {
            None
        }
    }
}

impl From<ServerName> for Server {
    fn from(name: ServerName) -> Self {
        Self {
            name,
            network: None,
        }
    }
}

impl fmt::Display for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(network) = &self.network {
            network.name.fmt(f)
        } else {
            self.name.fmt(f)
        }
    }
}

// here is a machine-readable (not-friendly) representation of a server, which
// can be hashed and used for history. Due to existing constraints this must be
// the server name if no bouncer network exists (due to the constraint of
// existing history).
impl fmt::Binary for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(network) = &self.network {
            // we insert a NUL to make sure no conflicts exist with existing networks
            write!(f, "\0{}", network.name)?;
        }
        Ok(())
    }
}

// Use case-insensitive comparison first, falling back to case-sensitive
// only when server names are equal (in a case-insensitive context).
impl Ord for Server {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let case_insensitive_ordering =
            self.name.to_lowercase().cmp(&other.name.to_lowercase());

        if !matches!(case_insensitive_ordering, cmp::Ordering::Equal) {
            return case_insensitive_ordering;
        }

        self.network.cmp(&other.network)
    }
}

impl PartialOrd for Server {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'de> Deserialize<'de> for Server {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct NameNetwork {
            name: ServerName,
            network: Option<Arc<BouncerNetwork>>,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Data {
            String(String),
            NameNetwork(NameNetwork),
        }

        let data = Data::deserialize(deserializer)?;

        Ok(match data {
            Data::String(name) => Server {
                name: name.into(),
                network: None,
            },
            Data::NameNetwork(NameNetwork { name, network }) => {
                Server { name, network }
            }
        })
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
pub struct ConfigMap(IndexMap<ServerName, Arc<config::Server>>);

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

impl ConfigMap {
    pub fn contains(&self, server: &ServerName) -> bool {
        self.0.contains_key(server)
    }
    pub async fn new(
        iter: impl IntoIterator<Item = (ServerName, config::Server)>,
    ) -> Result<Self, Error> {
        let mut map = IndexMap::new();
        for (i, (server, mut config)) in iter.into_iter().enumerate() {
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
            config.order = i as u16;
            map.insert(server, Arc::new(config));
        }

        Ok(Self(map))
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&ServerName, &Arc<config::Server>)> {
        self.0.iter()
    }
}

#[derive(Debug, Default)]
pub struct Map(IndexMap<Server, Arc<config::Server>>, sidebar::OrderBy);

impl From<ConfigMap> for Map {
    fn from(config: ConfigMap) -> Self {
        Self::new(config, OrderBy::Config)
    }
}

impl Map {
    pub fn new(config: ConfigMap, order: OrderBy) -> Self {
        let map = IndexMap::from_iter(
            config
                .0
                .into_iter()
                .map(|(server, config)| (Server::from(server), config)),
        );
        let mut s = Self(map, order);
        s.sort();
        s
    }

    fn comparator(
        &self,
    ) -> impl Fn(
        &Server,
        &Arc<config::Server>,
        &Server,
        &Arc<config::Server>,
    ) -> Ordering
    + use<> {
        fn cmp_alpha(
            s1: &Server,
            _: &Arc<config::Server>,
            s2: &Server,
            _: &Arc<config::Server>,
        ) -> Ordering {
            s1.cmp(s2)
        }

        fn cmp_config(
            s1: &Server,
            c1: &Arc<config::Server>,
            s2: &Server,
            c2: &Arc<config::Server>,
        ) -> Ordering {
            c1.order.cmp(&c2.order).then(cmp_alpha(s1, c1, s2, c2))
        }

        match self.1 {
            OrderBy::Alpha => cmp_alpha,
            OrderBy::Config => cmp_config,
        }
    }

    pub fn set_order(&mut self, order_by: OrderBy) {
        self.1 = order_by;
        self.sort();
    }

    pub fn sort(&mut self) {
        self.0.sort_by(self.comparator());
    }

    pub fn insert(&mut self, server: Server, config: Arc<config::Server>) {
        self.0.insert_sorted_by(server, config, self.comparator());
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

    pub fn get_mut(
        &mut self,
        server: &Server,
    ) -> Option<&mut Arc<config::Server>> {
        self.0.get_mut(server)
    }

    pub fn keys(&self) -> impl Iterator<Item = &Server> {
        self.0.keys()
    }

    pub fn entries(&self) -> impl Iterator<Item = Entry> + '_ {
        self.0.iter().map(Entry::from)
    }

    pub fn extract_if<F>(
        &mut self,
        pred: F,
    ) -> impl Iterator<Item = (Server, Arc<config::Server>)>
    where
        F: FnMut(&Server, &mut Arc<config::Server>) -> bool,
    {
        self.0.extract_if(0.., pred)
    }
}
