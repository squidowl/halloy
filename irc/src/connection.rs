use std::io;
use std::path::PathBuf;

use futures::stream::{SplitSink, SplitStream};
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::fs;
use tokio::net::TcpStream;
use tokio_native_tls::native_tls::{Certificate, Identity};
use tokio_native_tls::{native_tls, TlsConnector, TlsStream};
use tokio_util::codec::Framed;

use crate::{codec, Codec};

pub type Sender = SplitSink<Connection, proto::Message>;
pub type Receiver = SplitStream<Connection>;

pub enum Connection {
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
pub struct Config<'a> {
    pub server: &'a str,
    pub port: u16,
    pub security: Security<'a>,
}

impl Connection {
    pub async fn new(config: Config<'_>) -> Result<Self, Error> {
        let tcp = TcpStream::connect((config.server, config.port)).await?;

        if let Security::Secured {
            accept_invalid_certs,
            root_cert_path,
            client_cert_path,
            client_key_path,
        } = config.security
        {
            let mut builder = native_tls::TlsConnector::builder();
            builder.danger_accept_invalid_certs(accept_invalid_certs);

            if let Some(path) = root_cert_path {
                let bytes = fs::read(path).await?;
                let cert = Certificate::from_pem(&bytes)?;
                builder.add_root_certificate(cert);
            }

            if let (Some(cert_path), Some(pkcs8_key_path)) = (client_cert_path, client_key_path) {
                let cert_bytes = fs::read(cert_path).await?;
                let pkcs8_key_bytes = fs::read(pkcs8_key_path).await?;
                let identity = Identity::from_pkcs8(&cert_bytes, &pkcs8_key_bytes)?;
                builder.identity(identity);
            }

            let tls = TlsConnector::from(builder.build()?)
                .connect(config.server, tcp)
                .await?;

            Ok(Self::Tls(Framed::new(tls, Codec)))
        } else {
            Ok(Self::Unsecured(Framed::new(tcp, Codec)))
        }
    }

    pub fn split(self) -> (Sender, Receiver) {
        <Self as StreamExt>::split(self)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("tls error: {0}")]
    Tls(#[from] tokio_native_tls::native_tls::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

impl From<codec::Error> for Error {
    fn from(error: codec::Error) -> Self {
        match error {
            codec::Error::Io(error) => Error::Io(error),
        }
    }
}

macro_rules! delegate {
    ($e:expr, $($t:tt)*) => {
        match $e {
            $crate::connection::Connection::Tls(framed) => framed.$($t)*,
            $crate::connection::Connection::Unsecured(framed) => framed.$($t)*,
        }
    };
}

impl Stream for Connection {
    type Item = Result<Result<proto::Message, proto::parse::Error>, codec::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        delegate!(self.get_mut(), poll_next_unpin(cx))
    }
}

impl Sink<proto::Message> for Connection {
    type Error = codec::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        delegate!(self.get_mut(), poll_ready_unpin(cx))
    }

    fn start_send(
        self: std::pin::Pin<&mut Self>,
        message: proto::Message,
    ) -> Result<(), Self::Error> {
        delegate!(self.get_mut(), start_send_unpin(message))
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
