use std::collections::HashMap;
use std::num::NonZeroU16;
use std::path::PathBuf;
use std::time::Duration;

use irc::connection;
use serde::{Deserialize, Deserializer};
use tokio::fs;
use tokio::process::Command;

use self::filehost::Filehost;
use self::icon::Icon;
use crate::config::inclusivities::{
    Inclusivities, is_target_channel_included, is_target_query_included,
};
use crate::config::sidebar::OrderChannelsBy;
use crate::serde::{
    deserialize_path_buf_with_path_transformations,
    deserialize_path_buf_with_path_transformations_maybe,
    deserialize_u64_positive_integer,
};
use crate::{config, isupport, metadata, target};

pub mod filehost;
pub mod filters;
pub mod icon;
pub mod reroute;

pub use self::filters::{FancyRegex, Filters, Ignore};
pub use self::reroute::{Reroute, RerouteRule, RerouteTarget};

const DEFAULT_PORT: u16 = 6667;
const DEFAULT_TLS_PORT: u16 = 6697;
const DEFAULT_WS_PORT: u16 = 80;
const DEFAULT_WSS_PORT: u16 = 443;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Server {
    /// The client's nickname.
    #[serde(alias = "nick")]
    pub nickname: String,
    /// The client's NICKSERV password.
    pub nick_password: Option<String>,
    /// The client's NICKSERV password keyring entry.
    pub nick_password_keyring: config::keyring::Password,
    /// The client's NICKSERV password file.
    #[serde(
        deserialize_with = "deserialize_path_buf_with_path_transformations_maybe"
    )]
    pub nick_password_file: Option<PathBuf>,
    /// Truncate read from NICKSERV password file to first newline
    pub nick_password_file_first_line_only: bool,
    /// The client's NICKSERV password command.
    pub nick_password_command: Option<String>,
    /// The server's NICKSERV IDENTIFY syntax.
    pub nick_identify_syntax: Option<IdentifySyntax>,
    /// Alternative nicknames for the client, if the default is taken.
    pub alt_nicks: Vec<String>,
    /// The client's username (falls back to nickname if needed & not provided).
    pub username: Option<String>,
    /// The client's real name.
    pub realname: Option<String>,
    /// The server to connect to.
    pub server: String,
    /// The port to connect on.
    pub port: Option<NonZeroU16>,
    /// The password to connect to the server.
    pub password: Option<String>,
    /// The password keyring entry to connect to the server.
    pub password_keyring: config::keyring::Password,
    /// The file with the password to connect to the server.
    #[serde(
        deserialize_with = "deserialize_path_buf_with_path_transformations_maybe"
    )]
    pub password_file: Option<PathBuf>,
    /// Truncate read from password file to first newline
    pub password_file_first_line_only: bool,
    /// The command which outputs a password to connect to the server.
    pub password_command: Option<String>,
    /// Filter settings for the server, e.g. ignored nicks
    pub filters: Option<Filters>,
    /// Message reroute settings scoped to this server.
    pub reroute: Reroute,
    /// A list of channels to join on connection.
    pub channels: Vec<String>,
    /// A mapping of channel names to keys for join-on-connect.
    pub channel_keys: HashMap<String, String>,
    /// A mapping of channel names to keyring entries for join-on-connect.
    pub channel_keys_keyring: HashMap<String, config::keyring::Password>,
    /// Order server's channels
    pub order_channels_by: Option<OrderChannelsBy>,
    /// A list of queries to add to the sidebar on connection.
    pub queries: Vec<String>,
    /// The amount of inactivity in seconds before the client will ping the server.
    #[serde(deserialize_with = "deserialize_u64_positive_integer")]
    pub ping_time: u64,
    /// The amount of time in seconds for a client to reconnect due to no ping response.
    #[serde(deserialize_with = "deserialize_u64_positive_integer")]
    pub ping_timeout: u64,
    /// The amount of time in seconds before attempting to reconnect to the server when disconnected.
    #[serde(deserialize_with = "deserialize_duration_from_secs")]
    pub reconnect_delay: Duration,
    /// Whether the client should use NickServ GHOST to reclaim its primary nickname if it is in
    /// use. This has no effect if `nick_password` is not set.
    pub should_ghost: bool,
    /// The command(s) that should be sent to NickServ to recover a nickname. The nickname and
    /// password will be appended in that order after the command.
    /// E.g. `["RECOVER", "RELEASE"]` means `RECOVER nick pass` and `RELEASE nick pass` will be sent
    /// in that order.
    pub ghost_sequence: Vec<String>,
    /// User modestring to set on connect. Example: "+RB-x"
    pub umodes: Option<String>,
    /// Whether or not to use TLS.
    /// Clients will automatically panic if this is enabled without TLS support.
    pub use_tls: bool,
    /// Whether or not to connect using IRCv3 WebSocket transport.
    pub use_websocket: bool,
    /// The WebSocket request path.
    pub websocket_path: String,
    /// The WebSocket ping interval in seconds.
    #[serde(deserialize_with = "deserialize_who_poll_interval")]
    pub websocket_ping_interval: Duration,
    /// On `true`, all certificate validations are skipped. Defaults to `false`.
    pub dangerously_accept_invalid_certs: bool,
    /// The path to the root TLS certificate for this server in PEM format.
    #[serde(
        deserialize_with = "deserialize_path_buf_with_path_transformations_maybe"
    )]
    pub root_cert_path: Option<PathBuf>,
    /// Sasl authentication
    pub sasl: Option<Sasl>,
    /// Commands which are executed once connected.
    pub on_connect: Vec<String>,
    /// Enable WHO polling. Defaults to `true`.
    pub who_poll_enabled: bool,
    /// WHO poll interval for servers without away-notify.
    #[serde(deserialize_with = "deserialize_who_poll_interval")]
    pub who_poll_interval: Duration,
    /// A list of nicknames to monitor (if MONITOR is supported by the server).
    pub monitor: Vec<String>,
    pub chathistory: bool,
    #[serde(deserialize_with = "deserialize_anti_flood")]
    pub anti_flood: Duration,
    #[serde(skip)]
    pub order: u16,
    pub proxy: Option<config::Proxy>,
    pub confirm_message_delivery: ConfirmMessageDelivery,
    pub autoconnect: bool,
    pub typing: OptionalTyping,
    pub filehost: Filehost,
    pub metadata: HashMap<metadata::Key, String>,
    pub icon: Icon,
}

impl Server {
    pub fn new(
        server: String,
        port: Option<NonZeroU16>,
        nickname: String,
        channels: Vec<String>,
        use_tls: bool,
        use_websocket: bool,
    ) -> Self {
        Self {
            nickname,
            server,
            port: match port {
                None => Some(default_port(use_tls, use_websocket)),
                port => port,
            },
            channels,
            use_tls,
            use_websocket,
            dangerously_accept_invalid_certs: false,
            ..Default::default()
        }
    }

    pub fn connection(
        &self,
        proxy: Option<config::Proxy>,
    ) -> connection::Config<'_> {
        let security = if self.use_tls {
            connection::Security::Secured {
                accept_invalid_certs: self.dangerously_accept_invalid_certs,
                root_cert_path: self.root_cert_path.as_ref(),
                client_cert_path: self
                    .sasl
                    .as_ref()
                    .and_then(Sasl::external_cert),
                client_key_path: self
                    .sasl
                    .as_ref()
                    .and_then(Sasl::external_key),
            }
        } else {
            connection::Security::Unsecured
        };

        connection::Config {
            server: &self.server,
            port: match self.port {
                Some(port) => port,
                None => default_port(self.use_tls, self.use_websocket),
            }
            .get(),
            security,
            proxy: proxy.map(From::from),
            websocket: self.use_websocket.then_some(connection::WebSocket {
                path: &self.websocket_path,
                ping_interval: self.websocket_ping_interval,
            }),
        }
    }

    pub fn bouncer_config(&self) -> Self {
        Self {
            // nickserv info not relevant to the bounced network
            nick_password_keyring: config::keyring::Password::default(),
            nick_password_file: Option::default(),
            nick_password_command: Option::default(),
            nick_identify_syntax: Option::default(),

            // channel_keys not relevant
            channel_keys: HashMap::default(),
            channel_keys_keyring: HashMap::default(),

            // ghost sequence not relevant
            should_ghost: Default::default(),
            ghost_sequence: Server::default().ghost_sequence,

            ..self.clone()
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self {
            nickname: String::default(),
            nick_password: Option::default(),
            nick_password_keyring: config::keyring::Password::default(),
            nick_password_file: Option::default(),
            nick_password_file_first_line_only: true,
            nick_password_command: Option::default(),
            nick_identify_syntax: Option::default(),
            alt_nicks: Vec::default(),
            username: Option::default(),
            realname: Option::default(),
            server: String::default(),
            port: None,
            password: Option::default(),
            password_keyring: config::keyring::Password::default(),
            password_file: Option::default(),
            password_file_first_line_only: true,
            password_command: Option::default(),
            filters: Option::default(),
            reroute: Reroute::default(),
            channels: Vec::default(),
            channel_keys: HashMap::default(),
            channel_keys_keyring: HashMap::default(),
            order_channels_by: None,
            queries: Vec::default(),
            ping_time: 180,
            ping_timeout: 20,
            reconnect_delay: Duration::from_secs(10),
            should_ghost: Default::default(),
            ghost_sequence: vec!["REGAIN".into()],
            umodes: Option::default(),
            use_tls: true,
            use_websocket: false,
            websocket_path: "/".into(),
            websocket_ping_interval: Duration::from_secs(60),
            dangerously_accept_invalid_certs: Default::default(),
            root_cert_path: Option::default(),
            sasl: Option::default(),
            on_connect: Vec::default(),
            who_poll_enabled: true,
            who_poll_interval: Duration::from_secs(2),
            monitor: Vec::default(),
            chathistory: true,
            anti_flood: Duration::from_millis(2000),
            order: 0,
            proxy: None,
            confirm_message_delivery: ConfirmMessageDelivery::default(),
            autoconnect: true,
            typing: OptionalTyping::default(),
            filehost: Filehost::default(),
            metadata: HashMap::default(),
            icon: Icon::default(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentifySyntax {
    NickPassword,
    PasswordNick,
}

#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sasl {
    Plain {
        /// Account name (falls back to nickname if not provided)
        username: Option<String>,
        /// Account password,
        password: Option<String>,
        /// Account password keyring entry
        #[serde(default)]
        password_keyring: config::keyring::Password,
        /// Account password file
        #[serde(
            default,
            deserialize_with = "deserialize_path_buf_with_path_transformations_maybe"
        )]
        password_file: Option<PathBuf>,
        /// Truncate read from password file to first newline
        password_file_first_line_only: Option<bool>,
        /// Account password command
        password_command: Option<String>,
        /// Disconnect from server if SASL authentication fails. Defaults to `true`.
        disconnect_on_failure: Option<bool>,
    },
    External {
        /// The path to PEM encoded X509 user certificate for external auth
        #[serde(
            deserialize_with = "deserialize_path_buf_with_path_transformations"
        )]
        cert: PathBuf,
        /// The path to PEM encoded PKCS#8 private key corresponding to the user certificate for external auth
        #[serde(
            default,
            deserialize_with = "deserialize_path_buf_with_path_transformations_maybe"
        )]
        key: Option<PathBuf>,
        /// Disconnect from server if SASL authentication fails. Defaults to `true`.
        disconnect_on_failure: Option<bool>,
    },
}

impl Sasl {
    pub fn check_permissions(&self, server: &str) {
        match self {
            Sasl::Plain { password_file, .. } => {
                if let Some(pass_file) = password_file {
                    config::check_sensitive_file_permissions(
                        server,
                        pass_file,
                        "SASL password file",
                    );
                }
            }
            Sasl::External { cert, key, .. } => {
                config::check_sensitive_file_permissions(
                    server,
                    cert,
                    "SASL external cert",
                );

                if let Some(key) = key {
                    config::check_sensitive_file_permissions(
                        server,
                        key,
                        "SASL external key",
                    );
                }
            }
        }
    }

    pub async fn set_password(
        &mut self,
        server: &str,
        default_key: fn(&str) -> String,
        label: &'static str,
    ) -> Result<(), config::Error> {
        match self {
            Sasl::Plain {
                password: Some(_),
                password_keyring: config::keyring::Password::Disabled,
                password_file: None,
                password_command: None,
                ..
            } => {}
            Sasl::Plain {
                password: password @ None,
                password_keyring,
                password_file: None,
                password_command: None,
                ..
            } => {
                let Some(key) =
                    password_keyring.key_or_default(|| default_key(server))
                else {
                    return Err(config::Error::DuplicateSaslPassword);
                };

                let pass =
                    config::keyring::get_password(&key).await?.ok_or_else(
                        || config::Error::MissingKeyringPasswordEntry {
                            label: label.to_string(),
                            context: format!("server `{server}`"),
                            key: key.clone(),
                        },
                    )?;

                *password = Some(pass);
            }
            Sasl::Plain {
                password: password @ None,
                password_keyring: config::keyring::Password::Disabled,
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
                password_keyring: config::keyring::Password::Disabled,
                password_file: None,
                password_command: Some(pass_command),
                ..
            } => {
                let pass = read_from_command(pass_command).await?;

                *password = Some(pass);
            }
            Sasl::Plain { .. } => {
                return Err(config::Error::DuplicateSaslPassword);
            }
            Sasl::External { .. } => {}
        }

        Ok(())
    }

    pub fn disconnect_on_failure(&self) -> bool {
        match self {
            Sasl::Plain {
                disconnect_on_failure,
                ..
            }
            | Sasl::External {
                disconnect_on_failure,
                ..
            } => disconnect_on_failure.unwrap_or(true),
        }
    }

    pub fn command(&self) -> &'static str {
        match self {
            Sasl::Plain { .. } => "PLAIN",
            Sasl::External { .. } => "EXTERNAL",
        }
    }

    pub fn params(&self, nickname: &str) -> Vec<String> {
        const CHUNK_SIZE: usize = 400;

        match self {
            Sasl::Plain {
                username, password, ..
            } => {
                use base64::engine::Engine;

                let password = password
                    .as_ref()
                    .expect("SASL password must exist at this point!");

                // Exclude authorization ID, to use the authentication ID as the authorization ID
                // https://datatracker.ietf.org/doc/html/rfc4616#section-2
                let encoding = base64::engine::general_purpose::STANDARD
                    .encode(format!(
                        "\x00{}\x00{password}",
                        username
                            .as_ref()
                            .map_or(nickname, |username| username.as_str())
                    ));

                let chunks = encoding
                    .as_bytes()
                    .chunks(CHUNK_SIZE)
                    .collect::<Vec<&[u8]>>();

                let signal_end_of_response = chunks
                    .iter()
                    .last()
                    .is_none_or(|chunk| chunk.len() == CHUNK_SIZE);

                let mut params = chunks
                    .into_iter()
                    .map(|chunk| {
                        String::from_utf8(chunk.into())
                            .expect("chunks should be valid UTF-8")
                    })
                    .collect::<Vec<String>>();

                if signal_end_of_response {
                    params.push("+".into());
                }

                params
            }
            Sasl::External { .. } => vec!["+".into()],
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
            key.as_ref()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ConfirmMessageDelivery {
    pub enabled: bool,
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
}

impl Default for ConfirmMessageDelivery {
    fn default() -> Self {
        Self {
            enabled: true,
            exclude: None,
            include: None,
        }
    }
}

impl ConfirmMessageDelivery {
    pub fn is_target_channel_included(
        &self,
        channel: &target::Channel,
        server: &crate::server::Server,
        casemapping: isupport::CaseMap,
    ) -> bool {
        self.enabled
            && is_target_channel_included(
                self.include.as_ref(),
                self.exclude.as_ref(),
                None,
                channel,
                server,
                casemapping,
            )
    }

    pub fn is_target_query_included(
        &self,
        query: &target::Query,
        server: &crate::server::Server,
        casemapping: isupport::CaseMap,
    ) -> bool {
        self.enabled
            && is_target_query_included(
                self.include.as_ref(),
                self.exclude.as_ref(),
                query,
                server,
                casemapping,
            )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct OptionalTyping {
    pub share: Option<bool>,
    pub show: Option<bool>,
}

fn deserialize_anti_flood<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let milliseconds: u64 = Deserialize::deserialize(deserializer)?;

    if !(100..=60000).contains(&milliseconds) {
        Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(milliseconds),
            &"integer in the range 100 .. 60000",
        ))
    } else {
        Ok(Duration::from_millis(milliseconds))
    }
}

fn deserialize_who_poll_interval<'de, D>(
    deserializer: D,
) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let seconds: u64 = Deserialize::deserialize(deserializer)?;

    if !(1..=3600).contains(&seconds) {
        Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(seconds),
            &"integer in the range 1 .. 3600",
        ))
    } else {
        Ok(Duration::from_secs(seconds))
    }
}

fn deserialize_duration_from_secs<'de, D>(
    deserializer: D,
) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let seconds: u64 = Deserialize::deserialize(deserializer)?;

    if seconds == 0 {
        Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(seconds),
            &"any positive number of seconds",
        ))
    } else {
        Ok(Duration::from_secs(seconds))
    }
}

pub fn default_port(use_tls: bool, use_websocket: bool) -> NonZeroU16 {
    NonZeroU16::new(match (use_tls, use_websocket) {
        (true, true) => DEFAULT_WSS_PORT,
        (true, false) => DEFAULT_TLS_PORT,
        (false, true) => DEFAULT_WS_PORT,
        (false, false) => DEFAULT_PORT,
    })
    .expect("default ports are non-zero")
}

pub async fn read_from_command(
    pass_command: &str,
) -> Result<String, config::Error> {
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
        Err(config::Error::ExecutePasswordCommand(String::from_utf8(
            output.stderr,
        )?))
    }
}
