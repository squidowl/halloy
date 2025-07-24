use std::net::IpAddr;
use std::num::NonZeroU16;
use std::ops::RangeInclusive;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FileTransfer {
    /// Default directory to save files in. If not set, user will see a file dialog.
    pub save_directory: Option<PathBuf>,
    /// If true, act as the "client" for the transfer. Requires the remote user act as the server.
    pub passive: bool,
    /// Time in seconds to wait before timing out a transfer waiting to be accepted.
    pub timeout: u64,
    /// Auto-accept configuration for incoming file transfers.
    pub auto_accept: AutoAccept,
    pub server: Option<Server>,
}

impl Default for FileTransfer {
    fn default() -> Self {
        Self {
            save_directory: None,
            passive: true,
            timeout: 60 * 5,
            auto_accept: AutoAccept::default(),
            server: None,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AutoAccept {
    /// If true, automatically accept incoming file transfers. Requires save_directory to be set.
    pub enabled: bool,
    /// Auto-accept incoming file transfers from these nicks. Requires enabled to be true.
    pub nicks: Option<Vec<String>>,
    /// Auto-accept incoming file transfers from these masks (regex patterns). Requires enabled to be true.
    pub masks: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Server {
    /// Address advertised to the remote user to connect to
    pub public_address: IpAddr,
    /// Address to bind to when accepting connections
    pub bind_address: IpAddr,
    /// Port range used to bind with
    pub bind_ports: RangeInclusive<u16>,
}

impl<'de> Deserialize<'de> for Server {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Data {
            public_address: IpAddr,
            bind_address: IpAddr,
            bind_port_first: NonZeroU16,
            bind_port_last: NonZeroU16,
        }

        let Data {
            public_address,
            bind_address,
            bind_port_first,
            bind_port_last,
        } = Data::deserialize(deserializer)?;

        if bind_port_last < bind_port_first {
            return Err(serde::de::Error::custom(
                "`bind_port_last` must be greater than or equal to `bind_port_first`",
            ));
        }

        Ok(Server {
            public_address,
            bind_address,
            bind_ports: bind_port_first.get()..=bind_port_last.get(),
        })
    }
}
