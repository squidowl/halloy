use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Socks5,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Proxy {
    #[serde(rename = "type")]
    pub kind: Kind,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl From<Proxy> for irc::connection::Proxy {
    fn from(proxy: Proxy) -> irc::connection::Proxy {
        match proxy.kind {
            Kind::Socks5 => irc::connection::Proxy::Socks5 {
                host: proxy.host,
                port: proxy.port,
                username: proxy.username,
                password: proxy.password,
            },
        }
    }
}
