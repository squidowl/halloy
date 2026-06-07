use std::fmt;

use serde::Deserialize;

use crate::config::{self, keyring};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Proxy {
    Http {
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
        #[serde(default)]
        password_keyring: keyring::Password,
    },
    Socks5 {
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
        #[serde(default)]
        password_keyring: keyring::Password,
    },
    #[cfg(feature = "tor")]
    Tor,
}

impl Proxy {
    pub fn set_password(
        &mut self,
        default_key: impl Fn(&str) -> String,
        context: &str,
    ) -> Result<(), config::Error> {
        match self {
            Proxy::Http {
                password,
                password_keyring,
                ..
            } => set_password(
                password,
                password_keyring,
                default_key("http"),
                "HTTP proxy password",
                context,
            ),
            Proxy::Socks5 {
                password,
                password_keyring,
                ..
            } => set_password(
                password,
                password_keyring,
                default_key("socks5"),
                "SOCKS5 proxy password",
                context,
            ),
            #[cfg(feature = "tor")]
            Proxy::Tor => Ok(()),
        }
    }
}

impl From<Proxy> for irc::connection::Proxy {
    fn from(proxy: Proxy) -> irc::connection::Proxy {
        match proxy {
            Proxy::Http {
                host,
                port,
                username,
                password,
                password_keyring: _,
            } => irc::connection::Proxy::Http {
                host,
                port,
                username,
                password,
            },
            Proxy::Socks5 {
                host,
                port,
                username,
                password,
                password_keyring: _,
            } => irc::connection::Proxy::Socks5 {
                host,
                port,
                username,
                password,
            },
            #[cfg(feature = "tor")]
            Proxy::Tor => irc::connection::Proxy::Tor,
        }
    }
}

impl fmt::Display for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Proxy::Http { host, port, .. } => {
                write!(f, "http://{host}:{port}")
            }
            Proxy::Socks5 { host, port, .. } => {
                write!(f, "socks5://{host}:{port}")
            }
            #[cfg(feature = "tor")]
            Proxy::Tor => write!(f, "tor"),
        }
    }
}

pub fn build_client(
    proxy: Option<&Proxy>,
    identity: Option<reqwest::Identity>,
) -> Result<reqwest::Client, BuildError> {
    let mut builder = match proxy {
        None => reqwest::Client::builder(),
        Some(Proxy::Http {
            host,
            port,
            username,
            password,
            password_keyring: _,
        }) => {
            let mut proxy =
                reqwest::Proxy::all(format!("http://{host}:{port}"))?;

            if let Some(username) = username
                && let Some(password) = password
            {
                proxy = proxy.basic_auth(username, password);
            }

            reqwest::Client::builder().proxy(proxy)
        }
        Some(Proxy::Socks5 {
            host,
            port,
            username,
            password,
            password_keyring: _,
        }) => {
            let mut proxy =
                reqwest::Proxy::all(format!("socks5://{host}:{port}"))?;

            if let Some(username) = username
                && let Some(password) = password
            {
                proxy = proxy.basic_auth(username, password);
            }

            reqwest::Client::builder().proxy(proxy)
        }
        #[cfg(feature = "tor")]
        Some(Proxy::Tor) => {
            return Err(BuildError::Tor);
        }
    };

    if let Some(identity) = identity {
        builder = builder.identity(identity);
    }

    Ok(builder.build()?)
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("disabled when Tor proxy provided by Arti")]
    Tor,
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

fn set_password(
    password: &mut Option<String>,
    password_keyring: &keyring::Password,
    default_key: String,
    label: &'static str,
    context: &str,
) -> Result<(), config::Error> {
    let Some(key) = password_keyring.key_or_default(|| default_key) else {
        return Ok(());
    };

    if password.is_some() {
        return Err(config::Error::DuplicateProxyPassword {
            label: label.to_string(),
            context: context.to_string(),
        });
    }

    let pass = keyring::get_password(&key)?.ok_or_else(|| {
        config::Error::MissingKeyringPasswordEntry {
            label: label.to_string(),
            context: context.to_string(),
            key: key.clone(),
        }
    })?;

    *password = Some(pass);

    Ok(())
}
