use std::io;

use futures::stream::{SplitSink, SplitStream};
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::net::TcpStream;
use tokio_native_tls::{native_tls, TlsConnector, TlsStream};
use tokio_util::codec::Framed;

use crate::{codec, Codec};

pub type Sender = SplitSink<Connection, proto::Message>;
pub type Receiver = SplitStream<Connection>;

pub enum Connection {
    Tls(Framed<TlsStream<TcpStream>, Codec>),
    Unsecured(Framed<TcpStream, Codec>),
}

impl Connection {
    pub async fn new(server: &str, port: u16, use_tls: bool) -> Result<Self, Error> {
        let tcp = TcpStream::connect((server, port)).await?;

        if use_tls {
            let connector = native_tls::TlsConnector::builder().build()?;

            let tls = TlsConnector::from(connector).connect(server, tcp).await?;

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
