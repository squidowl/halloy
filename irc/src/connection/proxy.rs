use arti_client::{TorClient, TorClientConfig};
use async_http_proxy::{http_connect_tokio, http_connect_tokio_with_basic_auth};
use fast_socks5::client::{Config as Socks5Config, Socks5Stream};
use thiserror::Error;
use tokio::net::TcpStream;

use super::IrcStream;

#[derive(Debug, Clone)]
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

impl Proxy {
    pub async fn connect(&self, target_server: &str, target_port: u16) -> Result<IrcStream, Error> {
        match self {
            Proxy::Http {
                host,
                port,
                username,
                password,
            } => {
                connect_http(
                    host,
                    *port,
                    target_server,
                    target_port,
                    username.to_owned(),
                    password.to_owned(),
                )
                .await
            }
            Proxy::Socks5 {
                host,
                port,
                username,
                password,
            } => {
                connect_socks5(
                    host.to_string(),
                    *port,
                    target_server.to_string(),
                    target_port,
                    username.to_owned(),
                    password.to_owned(),
                )
                .await
            }
            Proxy::Tor => connect_tor(target_server.to_string(), target_port).await,
        }
    }
}

pub async fn connect_http(
    proxy_server: &str,
    proxy_port: u16,
    target_server: &str,
    target_port: u16,
    username: Option<String>,
    password: Option<String>,
) -> Result<IrcStream, Error> {
    let mut stream = TcpStream::connect((proxy_server, proxy_port)).await?;
    if let Some((username, password)) = username.zip(password) {
        http_connect_tokio_with_basic_auth(
            &mut stream,
            target_server,
            target_port,
            &username,
            &password,
        )
        .await?;
    } else {
        http_connect_tokio(&mut stream, target_server, target_port).await?;
    }
    Ok(IrcStream::Tcp(stream))
}

pub async fn connect_socks5(
    proxy_server: String,
    proxy_port: u16,
    target_server: String,
    target_port: u16,
    username: Option<String>,
    password: Option<String>,
) -> Result<IrcStream, Error> {
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

    Ok(IrcStream::Tcp(stream))
}

pub async fn connect_tor(target_server: String, target_port: u16) -> Result<IrcStream, Error> {
    let config = TorClientConfig::default();
    let tor_client = TorClient::create_bootstrapped(config).await?;

    let stream = tor_client.connect((target_server, target_port)).await?;

    Ok(IrcStream::Tor(stream))
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("http error: {0}")]
    Http(#[from] async_http_proxy::HttpError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("socks5 error: {0}")]
    Socks5(#[from] fast_socks5::SocksError),
    #[error("tor error: {0}")]
    Tor(#[from] arti_client::Error),
}
