use chrono::{DateTime, Utc};
use std::time::Duration;

use futures::channel::mpsc;
use futures::never::Never;
use futures::{stream, FutureExt, SinkExt, StreamExt};
use irc::proto::{self, command};
use irc::{codec, connection, Connection};
use tokio::time::{self, Instant, Interval};

use crate::client::Client;
use crate::server::Server;
use crate::time::Posix;
use crate::{config, message, server};

pub type Result<T = Update, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Connection(connection::Error),
}

#[derive(Debug)]
pub enum Update {
    Connected {
        server: Server,
        client: Client,
        is_initial: bool,
        sent_time: DateTime<Utc>,
    },
    Disconnected {
        server: Server,
        is_initial: bool,
        error: Option<String>,
        sent_time: DateTime<Utc>,
    },
    ConnectionFailed {
        server: Server,
        error: String,
        sent_time: DateTime<Utc>,
    },
    MessagesReceived(Server, Vec<message::Encoded>),
}

enum State {
    Disconnected {
        last_retry: Option<Instant>,
    },
    Connected {
        stream: Stream,
        batch: Batch,
        ping_time: Interval,
        ping_timeout: Option<Interval>,
    },
}

enum Input {
    IrcMessage(Result<codec::ParseResult, codec::Error>),
    Batch(Vec<message::Encoded>),
    Send(proto::Message),
    Ping,
    PingTimeout,
}

struct Stream {
    connection: Connection<irc::Codec>,
    receiver: mpsc::Receiver<proto::Message>,
}

pub async fn run(server: server::Entry, mut sender: mpsc::Sender<Update>) -> Never {
    let server::Entry { server, config } = server;

    let reconnect_delay = Duration::from_secs(config.reconnect_delay);

    let mut is_initial = true;
    let mut state = State::Disconnected { last_retry: None };

    // Notify app of initial disconnected state
    let _ = sender
        .send(Update::Disconnected {
            server: server.clone(),
            is_initial,
            error: None,
            sent_time: Utc::now(),
        })
        .await;

    loop {
        match &mut state {
            State::Disconnected { last_retry } => {
                if let Some(last_retry) = last_retry.as_ref() {
                    let remaining = reconnect_delay.saturating_sub(last_retry.elapsed());

                    if !remaining.is_zero() {
                        time::sleep(remaining).await;
                    }
                }

                match connect(server.clone(), config.clone()).await {
                    Ok((stream, client)) => {
                        log::info!("[{server}] connected");

                        let _ = sender
                            .send(Update::Connected {
                                server: server.clone(),
                                client,
                                is_initial,
                                sent_time: Utc::now(),
                            })
                            .await;

                        is_initial = false;

                        state = State::Connected {
                            stream,
                            batch: Batch::new(),
                            ping_timeout: None,
                            ping_time: ping_time_interval(config.ping_time),
                        };
                    }
                    Err(e) => {
                        let error = match e {
                            // unwrap Tls-specific error enums to access more error info
                            connection::Error::Tls(e) => format!("a TLS error occured: {e}"),
                            _ => e.to_string(),
                        };

                        log::warn!("[{server}] connection failed: {error}");

                        let _ = sender
                            .send(Update::ConnectionFailed {
                                server: server.clone(),
                                error,
                                sent_time: Utc::now(),
                            })
                            .await;

                        *last_retry = Some(Instant::now());
                    }
                }
            }
            State::Connected {
                stream,
                batch,
                ping_time,
                ping_timeout,
            } => {
                let input = {
                    let mut select = stream::select_all([
                        (&mut stream.connection).map(Input::IrcMessage).boxed(),
                        (&mut stream.receiver).map(Input::Send).boxed(),
                        ping_time.tick().into_stream().map(|_| Input::Ping).boxed(),
                        batch.map(Input::Batch).boxed(),
                    ]);

                    if let Some(timeout) = ping_timeout.as_mut() {
                        select.push(
                            timeout
                                .tick()
                                .into_stream()
                                .map(|_| Input::PingTimeout)
                                .boxed(),
                        );
                    }

                    select.next().await.expect("stream input")
                };

                match input {
                    Input::IrcMessage(Ok(Ok(message))) => match message.command {
                        proto::Command::PING(token) => {
                            let _ = stream.connection.send(command!("PONG", token)).await;
                        }
                        proto::Command::PONG(_, token) => {
                            let token = token.unwrap_or_default();
                            log::trace!("[{server}] pong received: {token}");

                            *ping_timeout = None;
                        }
                        proto::Command::ERROR(error) => {
                            log::warn!("[{server}] disconnected: {error}");
                            let _ = sender
                                .send(Update::Disconnected {
                                    server: server.clone(),
                                    is_initial,
                                    error: Some(error),
                                    sent_time: Utc::now(),
                                })
                                .await;
                            state = State::Disconnected {
                                last_retry: Some(Instant::now()),
                            };
                        }
                        _ => {
                            batch.messages.push(message.into());
                        }
                    },
                    Input::IrcMessage(Ok(Err(e))) => {
                        log::warn!("message decoding failed: {e}");
                    }
                    Input::IrcMessage(Err(e)) => {
                        log::warn!("[{server}] disconnected: {e}");
                        let _ = sender
                            .send(Update::Disconnected {
                                server: server.clone(),
                                is_initial,
                                error: Some(e.to_string()),
                                sent_time: Utc::now(),
                            })
                            .await;
                        state = State::Disconnected {
                            last_retry: Some(Instant::now()),
                        };
                    }
                    Input::Batch(messages) => {
                        let _ = sender
                            .send(Update::MessagesReceived(server.clone(), messages))
                            .await;
                    }
                    Input::Send(message) => {
                        let _ = stream.connection.send(message).await;
                    }
                    Input::Ping => {
                        let now = Posix::now().as_nanos().to_string();
                        log::trace!("[{server}] ping sent: {now}");

                        let _ = stream.connection.send(command!("PING", now)).await;

                        if ping_timeout.is_none() {
                            *ping_timeout = Some(ping_timeout_interval(config.ping_timeout));
                        }
                    }
                    Input::PingTimeout => {
                        log::warn!("[{server}] ping timeout");
                        let _ = sender
                            .send(Update::Disconnected {
                                server: server.clone(),
                                is_initial,
                                error: Some("ping timeout".into()),
                                sent_time: Utc::now(),
                            })
                            .await;
                        state = State::Disconnected {
                            last_retry: Some(Instant::now()),
                        };
                    }
                }
            }
        }
    }
}

async fn connect(
    server: Server,
    config: config::Server,
) -> Result<(Stream, Client), connection::Error> {
    let connection = Connection::new(config.connection(), irc::Codec).await?;

    let (sender, receiver) = mpsc::channel(100);

    Ok((
        Stream {
            connection,
            receiver,
        },
        Client::new(server, config, sender),
    ))
}

struct Batch {
    interval: Interval,
    messages: Vec<message::Encoded>,
}

impl Batch {
    const INTERVAL_MILLIS: u64 = 50;

    fn new() -> Self {
        Self {
            interval: time::interval_at(
                Instant::now() + Duration::from_millis(Self::INTERVAL_MILLIS),
                Duration::from_millis(Self::INTERVAL_MILLIS),
            ),
            messages: vec![],
        }
    }
}

impl futures::Stream for Batch {
    type Item = Vec<message::Encoded>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let batch = self.get_mut();

        match batch.interval.poll_tick(cx) {
            std::task::Poll::Ready(_) => {
                let messages = std::mem::take(&mut batch.messages);

                if messages.is_empty() {
                    std::task::Poll::Pending
                } else {
                    std::task::Poll::Ready(Some(messages))
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

fn ping_time_interval(secs: u64) -> Interval {
    time::interval_at(
        Instant::now() + Duration::from_secs(secs),
        Duration::from_secs(secs),
    )
}

fn ping_timeout_interval(secs: u64) -> Interval {
    time::interval_at(
        Instant::now() + Duration::from_secs(secs),
        Duration::from_secs(secs),
    )
}
