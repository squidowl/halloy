use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
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
                host: host,
                port: port,
                username: username,
                password: password,
            },
            Proxy::Socks5 {
                host,
                port,
                username,
                password,
            } => irc::connection::Proxy::Socks5 {
                host: host,
                port: port,
                username: username,
                password: password,
            },
            Proxy::Tor => irc::connection::Proxy::Tor,
        }
    }
}
