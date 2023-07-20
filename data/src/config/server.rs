use std::collections::HashMap;

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
    /// Whether or not to use TLS.
    /// Clients will automatically panic if this is enabled without TLS support.
    #[serde(default = "default_use_tls")]
    use_tls: bool,
    /// On `true`, all certificate validations are skipped. Defaults to `false`.
    #[serde(default)]
    dangerously_accept_invalid_certs: bool,
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
}

impl Server {
    pub fn connection(&self) -> connection::Config {
        let security = if self.use_tls {
            connection::Security::Secured {
                accept_invalid_certs: self.dangerously_accept_invalid_certs,
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

// TODO
/// The path to the TLS certificate for this server in DER format.
// pub cert_path: Option<String>,
// TODO
/// The path to a TLS certificate to use for CertFP client authentication in DER format.
// pub client_cert_path: Option<String>,
// TODO
/// The password for the certificate to use in CertFP authentication.
// pub client_cert_pass: Option<String>,
// TODO
/// The encoding type used for this connection.
/// This is typically UTF-8, but could be something else.
// pub encoding: Option<String>,
// TODO
/// The text that'll be sent in response to CTCP USERINFO requests.
// pub user_info: Option<String>,

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
