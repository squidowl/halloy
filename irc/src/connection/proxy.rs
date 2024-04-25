use fast_socks5::client::{Config as Socks5Config, Socks5Stream};
use thiserror::Error;
use tokio::net::TcpStream;

#[derive(Debug, Clone)]
pub enum Proxy {
    Socks5 {
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    },
}

pub async fn connect_socks5(
    proxy_server: String,
    proxy_port: u16,
    target_server: String,
    target_port: u16,
    username: Option<String>,
    password: Option<String>,
) -> Result<TcpStream, Error> {
    let stream = if let Some((username, password)) = username.zip(password) {
        Socks5Stream::connect_with_password(
            (proxy_server, proxy_port),
            target_server,
            target_port,
            username,
            password,
            Socks5Config::default(),
        )
        .await?
        .get_socket()
    } else {
        Socks5Stream::connect(
            (proxy_server, proxy_port),
            target_server,
            target_port,
            Socks5Config::default(),
        )
        .await?
        .get_socket()
    };

    Ok(stream)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("socks5 error: {0}")]
    Socks5(#[from] fast_socks5::SocksError),
}
