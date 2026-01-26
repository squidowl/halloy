use std::fmt;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Proxy {
    Http {
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    },
    Socks5 {
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    },
    #[cfg(feature = "tor")]
    Tor,
}

impl From<Proxy> for irc::connection::Proxy {
    fn from(proxy: Proxy) -> irc::connection::Proxy {
        match proxy {
            Proxy::Http {
                host,
                port,
                username,
                password,
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
