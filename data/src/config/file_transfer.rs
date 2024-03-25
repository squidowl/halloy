use std::{net::IpAddr, num::NonZeroU16, ops::RangeInclusive, path::PathBuf};

use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct FileTransfer {
    #[serde(
        default = "default_save_directory",
        deserialize_with = "deserialize_pathbuf_from_string"
    )]
    pub save_directory: PathBuf,
    #[serde(default = "default_passive")]
    pub passive: bool,
    /// Time in seconds to wait before timing out a transfer waiting to be accepted.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    pub bind: Option<Bind>,
}

impl Default for FileTransfer {
    fn default() -> Self {
        Self {
            save_directory: default_save_directory(),
            passive: default_passive(),
            timeout: default_timeout(),
            bind: None,
        }
    }
}

fn default_passive() -> bool {
    true
}

fn default_timeout() -> u64 {
    60 * 5
}

fn default_save_directory() -> PathBuf {
    dirs_next::download_dir().unwrap_or(PathBuf::from("/tmp/"))
}

#[derive(Debug, Clone)]
pub struct Bind {
    pub address: IpAddr,
    pub ports: RangeInclusive<u16>,
}

impl<'de> Deserialize<'de> for Bind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Data {
            address: IpAddr,
            port_first: NonZeroU16,
            port_last: NonZeroU16,
        }

        let Data {
            address,
            port_first,
            port_last,
        } = Data::deserialize(deserializer)?;

        if port_last < port_first {
            return Err(serde::de::Error::custom(
                "port_last must be greater than port_first",
            ));
        }

        Ok(Bind {
            address,
            ports: port_first.get()..=port_last.get(),
        })
    }
}

fn deserialize_pathbuf_from_string<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let path = String::deserialize(deserializer)?;
    Ok(PathBuf::from(path))
}
