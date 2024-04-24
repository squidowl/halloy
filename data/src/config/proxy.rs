use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Socks5,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Proxy {
    #[serde(rename = "type")]
    pub proxy_type: Kind,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

impl Into<irc::connection::Proxy> for Proxy {
    fn into(self) -> irc::connection::Proxy {
        match self.proxy_type {
            Kind::Socks5 => irc::connection::Proxy::Socks5 {
                host: self.host,
                port: self.port,
                username: self.username,
                password: self.password,
            },
        }
    }
}
