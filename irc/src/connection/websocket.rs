use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::BytesMut;
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{self, Message, Utf8Bytes, http};
use tokio_tungstenite::{WebSocketStream, client_async_with_config};
use tokio_util::codec::{Decoder, Encoder};
use tokio_util::either::Either;

use super::{IrcStream, Security, tls};

const BINARY_SUBPROTOCOL: &str = "binary.ircv3.net";
const TEXT_SUBPROTOCOL: &str = "text.ircv3.net";
const SUBPROTOCOLS: [&str; 2] = [BINARY_SUBPROTOCOL, TEXT_SUBPROTOCOL];
const MAX_IRC_WEBSOCKET_MESSAGE_SIZE: usize = 8192;

pub struct WebSocketConnection<Codec> {
    stream: WebSocketStream<WebSocketTransport>,
    codec: Codec,
    read: BytesMut,
    mode: Mode,
}

type WebSocketTransport = Either<IrcStream, TlsStream<IrcStream>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Binary,
    Text,
}

impl<Codec> WebSocketConnection<Codec> {
    pub async fn connect(
        stream: IrcStream,
        server: &str,
        port: u16,
        security: Security<'_>,
        path: &str,
        codec: Codec,
    ) -> Result<Self, Error> {
        let (scheme, transport) = match security {
            Security::Unsecured => ("ws", Either::Left(stream)),
            Security::Secured {
                accept_invalid_certs,
                root_cert_path,
                client_cert_path,
                client_key_path,
            } => {
                let stream = tls::connect(
                    stream,
                    server,
                    accept_invalid_certs,
                    root_cert_path,
                    client_cert_path,
                    client_key_path,
                )
                .await?;

                ("wss", Either::Right(stream))
            }
        };

        let mut request =
            websocket_uri(scheme, server, port, path).into_client_request()?;
        request.headers_mut().insert(
            http::header::SEC_WEBSOCKET_PROTOCOL,
            http::HeaderValue::from_str(&requested_subprotocols())
                .expect("subprotocols should be valid header value"),
        );

        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_IRC_WEBSOCKET_MESSAGE_SIZE))
            .max_frame_size(Some(MAX_IRC_WEBSOCKET_MESSAGE_SIZE));

        let (stream, response) =
            client_async_with_config(request, transport, Some(config)).await?;

        Ok(Self {
            stream,
            codec,
            read: BytesMut::new(),
            mode: mode_from_response(&response)?,
        })
    }

    pub async fn shutdown(mut self) -> Result<(), Error> {
        self.stream.close(None).await?;
        Ok(())
    }

    // websocket frames don't include CRLF, so we re-add it for our decoder
    fn push_crlf_line(&mut self, bytes: &[u8]) {
        self.read.extend_from_slice(bytes);
        if !self.read.ends_with(b"\r\n") {
            self.read.extend_from_slice(b"\r\n");
        }
    }
}

impl<Codec> Stream for WebSocketConnection<Codec>
where
    Codec: Decoder + Unpin,
    Codec::Error: From<io::Error>,
{
    type Item = Result<Codec::Item, Codec::Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            if let Some(item) = this.codec.decode(&mut this.read)? {
                return Poll::Ready(Some(Ok(item)));
            }

            match this.stream.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(Message::Text(text)))) => {
                    this.push_crlf_line(text.as_ref());
                }
                Poll::Ready(Some(Ok(Message::Binary(bytes)))) => {
                    this.push_crlf_line(&bytes);
                }
                Poll::Ready(Some(Ok(Message::Close(_)))) => {
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Ok(_))) => {}
                Poll::Ready(Some(Err(error))) => {
                    return Poll::Ready(Some(Err(websocket_error(error))));
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<Item, Codec> Sink<Item> for WebSocketConnection<Codec>
where
    Codec: Encoder<Item> + Unpin,
    Codec::Error: From<io::Error>,
{
    type Error = Codec::Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .stream
            .poll_ready_unpin(cx)
            .map_err(websocket_error)
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.get_mut();
        let mut bytes = BytesMut::new();

        this.codec.encode(item, &mut bytes)?;
        // websocket frames are already message boundaries, so we strip CRLF
        if bytes.ends_with(b"\r\n") {
            bytes.truncate(bytes.len() - 2);
        }

        let bytes = bytes.freeze();
        let message = match this.mode {
            Mode::Binary => Message::Binary(bytes),
            Mode::Text => {
                let text = Utf8Bytes::try_from(bytes).map_err(|error| {
                    io::Error::new(io::ErrorKind::InvalidData, error)
                })?;
                Message::Text(text)
            }
        };

        this.stream
            .start_send_unpin(message)
            .map_err(websocket_error)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .stream
            .poll_flush_unpin(cx)
            .map_err(websocket_error)
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .stream
            .poll_close_unpin(cx)
            .map_err(websocket_error)
    }
}

fn websocket_uri(scheme: &str, server: &str, port: u16, path: &str) -> String {
    let host = if server.contains(':') && !server.starts_with('[') {
        format!("[{server}]")
    } else {
        server.to_string()
    };
    let path = if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };

    format!("{scheme}://{host}:{port}{path}")
}

fn requested_subprotocols() -> String {
    SUBPROTOCOLS.join(", ")
}

fn mode_from_response(
    response: &http::Response<Option<Vec<u8>>>,
) -> Result<Mode, Error> {
    let protocol = response
        .headers()
        .get(http::header::SEC_WEBSOCKET_PROTOCOL)
        .and_then(|value| value.to_str().ok());

    match protocol {
        Some(TEXT_SUBPROTOCOL) => Ok(Mode::Text),
        Some(BINARY_SUBPROTOCOL) | None => Ok(Mode::Binary),
        Some(protocol) => Err(Error::UnsupportedSubprotocol(protocol.into())),
    }
}

fn websocket_error<E>(error: tungstenite::Error) -> E
where
    E: From<io::Error>,
{
    io::Error::new(io::ErrorKind::ConnectionAborted, error).into()
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("tls error: {0}")]
    Tls(#[from] tls::Error),
    #[error("websocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),
    #[error("unsupported websocket subprotocol: {0}")]
    UnsupportedSubprotocol(String),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}
