use std::collections::{BTreeMap, HashMap};
use std::fmt;

use serde::{Deserialize, Serialize};

const VERSION: &str = include_str!("../../VERSION");

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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    /// The client's nickname.
    pub nickname: Option<String>,
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
    pub server: Option<String>,
    /// The port to connect on.
    pub port: Option<u16>,
    /// The password to connect to the server.
    pub password: Option<String>,
    /// Whether or not to use TLS.
    /// Clients will automatically panic if this is enabled without TLS support.
    pub use_tls: Option<bool>,
    /// The path to the TLS certificate for this server in DER format.
    pub cert_path: Option<String>,
    /// The path to a TLS certificate to use for CertFP client authentication in DER format.
    pub client_cert_path: Option<String>,
    /// The password for the certificate to use in CertFP authentication.
    pub client_cert_pass: Option<String>,
    /// On `true`, all certificate validations are skipped. Defaults to `false`.
    pub dangerously_accept_invalid_certs: Option<bool>,
    /// The encoding type used for this connection.
    /// This is typically UTF-8, but could be something else.
    pub encoding: Option<String>,
    /// A list of channels to join on connection.
    #[serde(default)]
    pub channels: Vec<String>,
    /// User modes to set on connect. Example: "+RB -x"
    pub umodes: Option<String>,
    /// The text that'll be sent in response to CTCP USERINFO requests.
    pub user_info: Option<String>,
    /// The amount of inactivity in seconds before the client will ping the server.
    pub ping_time: Option<u32>,
    /// The amount of time in seconds for a client to reconnect due to no ping response.
    pub ping_timeout: Option<u32>,
    /// The length in seconds of a rolling window for message throttling. If more than
    /// `max_messages_in_burst` messages are sent within `burst_window_length` seconds, additional
    /// messages will be delayed automatically as appropriate. In particular, in the past
    /// `burst_window_length` seconds, there will never be more than `max_messages_in_burst` messages
    /// sent.
    pub burst_window_length: Option<u32>,
    /// The maximum number of messages that can be sent in a burst window before they'll be delayed.
    /// Messages are automatically delayed as appropriate.
    pub max_messages_in_burst: Option<u32>,
    /// Whether the client should use NickServ GHOST to reclaim its primary nickname if it is in
    /// use. This has no effect if `nick_password` is not set.
    #[serde(default)]
    pub should_ghost: bool,
    /// The command(s) that should be sent to NickServ to recover a nickname. The nickname and
    /// password will be appended in that order after the command.
    /// E.g. `["RECOVER", "RELEASE"]` means `RECOVER nick pass` and `RELEASE nick pass` will be sent
    /// in that order.
    pub ghost_sequence: Option<Vec<String>>,
    /// A mapping of channel names to keys for join-on-connect.
    #[serde(default)]
    pub channel_keys: HashMap<String, String>,
}

impl From<Config> for irc::client::data::Config {
    fn from(config: Config) -> Self {
        irc::client::data::Config {
            nickname: config.nickname,
            nick_password: config.nick_password,
            alt_nicks: config.alt_nicks,
            username: config.username,
            realname: config.realname,
            server: config.server,
            port: config.port,
            password: config.password,
            use_tls: config.use_tls,
            cert_path: config.cert_path,
            client_cert_path: config.client_cert_path,
            client_cert_pass: config.client_cert_pass,
            dangerously_accept_invalid_certs: config.dangerously_accept_invalid_certs,
            encoding: config.encoding,
            channels: config.channels,
            umodes: config.umodes,
            user_info: config.user_info,
            ping_time: config.ping_time,
            ping_timeout: config.ping_timeout,
            burst_window_length: config.burst_window_length,
            max_messages_in_burst: config.max_messages_in_burst,
            should_ghost: config.should_ghost,
            ghost_sequence: config.ghost_sequence,
            channel_keys: config.channel_keys,
            version: Some(format!("Halloy {VERSION}")),
            owners: vec![],
            source: None,
            use_mock_connection: false,
            mock_initial_value: None,
            options: HashMap::new(),
            path: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub server: Server,
    pub config: Config,
}

impl<'a> From<(&'a Server, &'a Config)> for Entry {
    fn from((server, config): (&'a Server, &'a Config)) -> Self {
        Self {
            server: server.clone(),
            config: config.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Map(BTreeMap<Server, Config>);

impl Map {
    pub fn remove(&mut self, server: &Server) {
        self.0.remove(server);
    }

    pub fn entries(&self) -> impl Iterator<Item = Entry> + '_ {
        self.0.iter().map(Entry::from)
    }
}
