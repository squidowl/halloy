use std::net::IpAddr;
use std::path::PathBuf;

use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::client::TlsStream;
use tokio_socks::tcp::Socks5Stream;
use tokio_util::codec;
use tokio_util::codec::Framed;

mod tls;

pub enum Connection<Codec> {
    Tls(Framed<TlsStream<TcpStream>, Codec>),
    Unsecured(Framed<TcpStream, Codec>),
}

#[derive(Debug, Clone)]
pub enum Security<'a> {
    Unsecured,
    Secured {
        accept_invalid_certs: bool,
        root_cert_path: Option<&'a PathBuf>,
        client_cert_path: Option<&'a PathBuf>,
        client_key_path: Option<&'a PathBuf>,
    },
}

#[derive(Debug, Clone)]
pub enum Proxy {
    Socks5 {
        host: String,
        port: u16,
        username: String,
        password: String,
    },
}

#[derive(Debug, Clone)]
pub struct Config<'a> {
    pub server: &'a str,
    pub port: u16,
    pub security: Security<'a>,
    pub proxy: Option<Proxy>,
}

impl<Codec> Connection<Codec> {
    pub async fn new(config: Config<'_>, codec: Codec) -> Result<Self, Error> {
        let target = (config.server, config.port);
        let tcp = match config.proxy {
            None => TcpStream::connect(target).await?,
            Some(Proxy::Socks5 {
                host,
                port,
                username,
                password,
            }) => {
                let proxy = (host.as_str(), port);
                if username.trim().is_empty() {
                    Socks5Stream::connect(proxy, target).await?.into_inner()
                } else {
                    Socks5Stream::connect_with_password(proxy, target, &username, &password)
                        .await?
                        .into_inner()
                }
            }
        };

        if let Security::Secured {
            accept_invalid_certs,
            root_cert_path,
            client_cert_path,
            client_key_path,
        } = config.security
        {
            let tls = tls::connect(
                tcp,
                config.server,
                accept_invalid_certs,
                root_cert_path,
                client_cert_path,
                client_key_path,
            )
            .await?;

            Ok(Self::Tls(Framed::new(tls, codec)))
        } else {
            Ok(Self::Unsecured(Framed::new(tcp, codec)))
        }
    }

    /// Binds a listener and returns a single connection
    /// once accepted. Useful for DCC flow.
    pub async fn listen_and_accept(
        address: IpAddr,
        port: u16,
        security: Security<'_>,
        codec: Codec,
    ) -> Result<Self, Error> {
        let listener = TcpListener::bind((address, port)).await?;

        let (tcp, _remote) = listener.accept().await?;

        match security {
            Security::Unsecured => Ok(Self::Unsecured(Framed::new(tcp, codec))),
            Security::Secured { .. } => {
                todo!();
            }
        }
    }

    pub async fn shutdown(self) -> Result<(), Error> {
        match self {
            Connection::Tls(framed) => {
                framed.into_inner().shutdown().await?;
            }
            Connection::Unsecured(framed) => {
                framed.into_inner().shutdown().await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("tls error: {0}")]
    Tls(#[from] tls::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("proxy error: {0}")]
    Proxy(#[from] tokio_socks::Error),
}

macro_rules! delegate {
    ($e:expr, $($t:tt)*) => {
        match $e {
            $crate::connection::Connection::Tls(framed) => framed.$($t)*,
            $crate::connection::Connection::Unsecured(framed) => framed.$($t)*,
        }
    };
}

impl<Codec> Stream for Connection<Codec>
where
    Codec: codec::Decoder,
{
    type Item = Result<Codec::Item, Codec::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        delegate!(self.get_mut(), poll_next_unpin(cx))
    }
}

impl<Item, Codec> Sink<Item> for Connection<Codec>
where
    Codec: codec::Encoder<Item>,
{
    type Error = Codec::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        delegate!(self.get_mut(), poll_ready_unpin(cx))
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        delegate!(self.get_mut(), start_send_unpin(item))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        delegate!(self.get_mut(), poll_flush_unpin(cx))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        delegate!(self.get_mut(), poll_close_unpin(cx))
    }
}
