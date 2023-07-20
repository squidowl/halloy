use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Server {
    /// The client's nickname.
    pub nickname: String,
    // TODO
    /// The client's NICKSERV password.
    // pub nick_password: Option<String>,
    /// Alternative nicknames for the client, if the default is taken.
    // TODO
    // #[serde(default)]
    // pub alt_nicks: Vec<String>,
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
    /// Whether or not to use TLS.
    /// Clients will automatically panic if this is enabled without TLS support.
    #[serde(default = "default_use_tls")]
    pub use_tls: bool,
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
/// On `true`, all certificate validations are skipped. Defaults to `false`.
// pub dangerously_accept_invalid_certs: Option<bool>,
// TODO
/// The encoding type used for this connection.
/// This is typically UTF-8, but could be something else.
// pub encoding: Option<String>,
// TODO
/// A list of channels to join on connection.
// #[serde(default)]
// pub channels: Vec<String>,
// TODO
/// User modes to set on connect. Example: "+RB -x"
// pub umodes: Option<String>,
// TODO
/// The text that'll be sent in response to CTCP USERINFO requests.
// pub user_info: Option<String>,
/// The amount of inactivity in seconds before the client will ping the server.
// TODO
// #[serde(default = "default_ping_time")]
// pub ping_time: u32,
// TODO
// /// The amount of time in seconds for a client to reconnect due to no ping response.
// #[serde(default = "default_ping_timeout")]
// pub ping_timeout: u32,
// TODO
/// Whether the client should use NickServ GHOST to reclaim its primary nickname if it is in
/// use. This has no effect if `nick_password` is not set.
// #[serde(default)]
// pub should_ghost: bool,
// TODO
/// The command(s) that should be sent to NickServ to recover a nickname. The nickname and
/// password will be appended in that order after the command.
/// E.g. `["RECOVER", "RELEASE"]` means `RECOVER nick pass` and `RELEASE nick pass` will be sent
/// in that order.
// pub ghost_sequence: Option<Vec<String>>,
// TODO
/// A mapping of channel names to keys for join-on-connect.
// #[serde(default)]
// pub channel_keys: HashMap<String, String>,

fn default_use_tls() -> bool {
    true
}

fn default_port() -> u16 {
    6697
}

fn default_ping_time() -> u32 {
    180
}

fn default_ping_timeout() -> u32 {
    20
}
