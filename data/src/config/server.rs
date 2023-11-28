use std::collections::HashMap;
use std::path::PathBuf;

use irc::connection;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Server {
    /// The client's nickname.
    pub nickname: String,
    /// The client's NICKSERV password.
    pub nick_password: Option<String>,
    /// Alternative nicknames for the client, if the default is taken.
    #[serde(default)]
    pub alt_nicks: Vec<String>,
    /// The client's username.
    pub username: Option<String>,
    /// The client's real name.
    pub realname: Option<String>,
    /// The server to connect to.
    pub server: String,
    /// The port to connect on.
    #[serde(default = "default_port")]
    pub port: u16,
    /// The password to connect to the server.
    pub password: Option<String>,
    /// A list of channels to join on connection.
    #[serde(default)]
    pub channels: Vec<String>,
    /// A mapping of channel names to keys for join-on-connect.
    #[serde(default)]
    pub channel_keys: HashMap<String, String>,
    /// The amount of inactivity in seconds before the client will ping the server.
    #[serde(default = "default_ping_time")]
    pub ping_time: u64,
    /// The amount of time in seconds for a client to reconnect due to no ping response.
    #[serde(default = "default_ping_timeout")]
    pub ping_timeout: u64,
    /// The amount of time in seconds before attempting to reconnect to the server when disconnected.
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay: u64,
    /// Whether the client should use NickServ GHOST to reclaim its primary nickname if it is in
    /// use. This has no effect if `nick_password` is not set.
    #[serde(default)]
    pub should_ghost: bool,
    /// The command(s) that should be sent to NickServ to recover a nickname. The nickname and
    /// password will be appended in that order after the command.
    /// E.g. `["RECOVER", "RELEASE"]` means `RECOVER nick pass` and `RELEASE nick pass` will be sent
    /// in that order.
    #[serde(default = "default_ghost_sequence")]
    pub ghost_sequence: Vec<String>,
    /// User modestring to set on connect. Example: "+RB-x"
    pub umodes: Option<String>,
    /// Whether or not to use TLS.
    /// Clients will automatically panic if this is enabled without TLS support.
    #[serde(default = "default_use_tls")]
    use_tls: bool,
    /// On `true`, all certificate validations are skipped. Defaults to `false`.
    #[serde(default)]
    dangerously_accept_invalid_certs: bool,
    /// The path to the root TLS certificate for this server in PEM format.
    root_cert_path: Option<PathBuf>,
    /// Sasl authentication
    pub sasl: Option<Sasl>,
    /// Commands which are executed once connected.
    #[serde(default)]
    pub on_connect: Vec<String>,
}

impl Server {
    pub fn connection(&self) -> connection::Config {
        let security = if self.use_tls {
            connection::Security::Secured {
                accept_invalid_certs: self.dangerously_accept_invalid_certs,
                root_cert_path: self.root_cert_path.as_ref(),
                client_cert_path: self.sasl.as_ref().and_then(Sasl::external_cert),
                client_key_path: self.sasl.as_ref().and_then(Sasl::external_key),
            }
        } else {
            connection::Security::Unsecured
        };

        connection::Config {
            server: &self.server,
            port: self.port,
            security,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sasl {
    Plain {
        /// Account name
        username: String,
        /// Account password,
        password: String,
    },
    External {
        /// The path to PEM encoded X509 user certificate for external auth
        cert: PathBuf,
        /// The path to PEM encoded PKCS#8 private key corresponding to the user certificate for external auth
        key: PathBuf,
    },
}

impl Sasl {
    pub fn command(&self) -> &'static str {
        match self {
            Sasl::Plain { .. } => "PLAIN",
            Sasl::External { .. } => "EXTERNAL",
        }
    }

    pub fn param(&self) -> String {
        match self {
            Sasl::Plain { username, password } => {
                use base64::engine::Engine;
                base64::engine::general_purpose::STANDARD
                    .encode(format!("{username}\x00{username}\x00{password}"))
            }
            Sasl::External { .. } => "+".into(),
        }
    }

    fn external_cert(&self) -> Option<&PathBuf> {
        if let Self::External { cert, .. } = self {
            Some(cert)
        } else {
            None
        }
    }

    fn external_key(&self) -> Option<&PathBuf> {
        if let Self::External { key, .. } = self {
            Some(key)
        } else {
            None
        }
    }
}

fn default_use_tls() -> bool {
    true
}

fn default_port() -> u16 {
    6697
}

fn default_ping_time() -> u64 {
    180
}

fn default_ping_timeout() -> u64 {
    20
}

fn default_reconnect_delay() -> u64 {
    10
}

fn default_ghost_sequence() -> Vec<String> {
    vec!["GHOST".into()]
}
