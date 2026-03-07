use std::path::PathBuf;

use serde::Deserialize;
use tokio::fs;

use crate::environment;

const NETWORKS_FILE_NAME: &str = "networks.toml";
const SUPPORTED_VERSION: u16 = 1;

#[derive(Debug, Clone, Default)]
pub struct Map {
    entries: Vec<Entry>,
}

#[derive(Debug, Clone)]
struct Entry {
    networks: Vec<String>,
    hosts: Vec<String>,
}

impl Map {
    pub fn path() -> PathBuf {
        environment::config_dir().join(NETWORKS_FILE_NAME)
    }

    pub async fn load() -> Result<Self, Error> {
        let path = Self::path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).await?;
        Self::parse(content.as_str())
    }

    pub fn network_names_for_host<'a>(&'a self, host: &str) -> Vec<&'a str> {
        let host = normalize_host(host);
        let mut networks = Vec::new();
        let mut known_networks = Vec::<String>::new();

        for entry in &self.entries {
            if entry.hosts.iter().any(|known| known == &host) {
                for network in &entry.networks {
                    let normalized = network.to_ascii_lowercase();

                    if !known_networks.contains(&normalized) {
                        known_networks.push(normalized);
                        networks.push(network.as_str());
                    }
                }
            }
        }

        networks
    }

    fn parse(content: &str) -> Result<Self, Error> {
        let file = toml::from_str::<File>(content)?;

        if file.version != SUPPORTED_VERSION {
            return Err(Error::UnsupportedVersion(file.version));
        }

        let entries = file
            .networks
            .into_iter()
            .map(|entry| Entry {
                networks: entry.network.into_vec(),
                hosts: entry
                    .hosts
                    .into_iter()
                    .map(|host| normalize_host(host.as_str()))
                    .filter(|host| !host.is_empty())
                    .collect(),
            })
            .filter(|entry| {
                !entry.networks.is_empty() && !entry.hosts.is_empty()
            })
            .collect();

        Ok(Self { entries })
    }
}

fn normalize_host(host: &str) -> String {
    host.trim_end_matches('.').to_ascii_lowercase()
}

#[derive(Debug, Deserialize)]
struct File {
    version: u16,
    #[serde(default)]
    networks: Vec<RawEntry>,
}

#[derive(Debug, Deserialize)]
struct RawEntry {
    network: NetworkName,
    hosts: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NetworkName {
    Single(String),
    Multiple(Vec<String>),
}

impl NetworkName {
    fn into_vec(self) -> Vec<String> {
        match self {
            NetworkName::Single(network) => {
                if network.trim().is_empty() {
                    Vec::new()
                } else {
                    vec![network]
                }
            }
            NetworkName::Multiple(networks) => networks
                .into_iter()
                .filter(|network| !network.trim().is_empty())
                .collect(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] toml::de::Error),
    #[error("unsupported networks.toml version: {0}")]
    UnsupportedVersion(u16),
}

#[cfg(test)]
mod tests {
    use super::Map;

    #[test]
    fn parses_network_as_string_and_array() {
        let map = Map::parse(
            r#"
version = 1

[[networks]]
network = "DALnet"
hosts = ["irc.dal.net"]

[[networks]]
network = ["Libera.Chat", "Libera"]
hosts = ["irc.libera.chat"]
"#,
        )
        .unwrap();

        let dal = map.network_names_for_host("irc.dal.net");
        let libera = map.network_names_for_host("irc.libera.chat");

        assert_eq!(dal, vec!["DALnet"]);
        assert_eq!(libera, vec!["Libera.Chat", "Libera"]);
    }

    #[test]
    fn host_lookup_is_case_insensitive() {
        let map = Map::parse(
            r#"
version = 1

[[networks]]
network = "Libera.Chat"
hosts = ["IRC.Libera.Chat"]
"#,
        )
        .unwrap();

        let networks = map.network_names_for_host("irc.libera.chat");

        assert_eq!(networks, vec!["Libera.Chat"]);
    }
}
