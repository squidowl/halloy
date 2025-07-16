use std::net::IpAddr;
use std::path::PathBuf;
use std::pin::Pin;

#[cfg(feature = "tor")]
use arti_client::DataStream as TorStream;
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::client::TlsStream;
use tokio_util::codec;
use tokio_util::codec::Framed;

pub use self::proxy::Proxy;

mod proxy;
mod tls;

pub enum IrcStream {
    Tcp(TcpStream),
    #[cfg(feature = "tor")]
    Tor(TorStream),
}

pub enum Connection<Codec> {
    Tls(Framed<TlsStream<IrcStream>, Codec>),
    Unsecured(Framed<IrcStream, Codec>),
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
pub struct Config<'a> {
    pub server: &'a str,
    pub port: u16,
    pub security: Security<'a>,
    pub proxy: Option<Proxy>,
}

impl<Codec> Connection<Codec> {
    pub async fn new(config: Config<'_>, codec: Codec) -> Result<Self, Error> {
        let stream = match config.proxy {
            None => IrcStream::Tcp(
                TcpStream::connect((config.server, config.port)).await?,
            ),
            Some(proxy) => proxy.connect(config.server, config.port).await?,
        };

        if let Security::Secured {
            accept_invalid_certs,
            root_cert_path,
            client_cert_path,
            client_key_path,
        } = config.security
        {
            let tls = tls::connect(
                stream,
                config.server,
                accept_invalid_certs,
                root_cert_path,
                client_cert_path,
                client_key_path,
            )
            .await?;

            Ok(Self::Tls(Framed::new(tls, codec)))
        } else {
            Ok(Self::Unsecured(Framed::new(stream, codec)))
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
        let stream = IrcStream::Tcp(tcp);

        match security {
            Security::Unsecured => {
                Ok(Self::Unsecured(Framed::new(stream, codec)))
            }
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
    Proxy(#[from] proxy::Error),
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

    fn start_send(
        self: std::pin::Pin<&mut Self>,
        item: Item,
    ) -> Result<(), Self::Error> {
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

impl AsyncRead for IrcStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            IrcStream::Tcp(s) => Pin::new(s).poll_read(cx, buf),
            #[cfg(feature = "tor")]
            IrcStream::Tor(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for IrcStream {
    fn is_write_vectored(&self) -> bool {
        match self {
            IrcStream::Tcp(s) => s.is_write_vectored(),
            #[cfg(feature = "tor")]
            IrcStream::Tor(s) => s.is_write_vectored(),
        }
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            IrcStream::Tcp(s) => Pin::new(s).poll_flush(cx),
            #[cfg(feature = "tor")]
            IrcStream::Tor(s) => Pin::new(s).poll_flush(cx),
        }
    }
    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            IrcStream::Tcp(s) => Pin::new(s).poll_shutdown(cx),
            #[cfg(feature = "tor")]
            IrcStream::Tor(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            IrcStream::Tcp(s) => Pin::new(s).poll_write(cx, buf),
            #[cfg(feature = "tor")]
            IrcStream::Tor(s) => Pin::new(s).poll_write(cx, buf),
        }
    }
    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            IrcStream::Tcp(s) => Pin::new(s).poll_write_vectored(cx, bufs),
            #[cfg(feature = "tor")]
            IrcStream::Tor(s) => Pin::new(s).poll_write_vectored(cx, bufs),
        }
    }
}
