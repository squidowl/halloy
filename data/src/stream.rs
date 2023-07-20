use std::time::Duration;

use futures::channel::mpsc;
use futures::never::Never;
use futures::{stream, SinkExt, StreamExt};
use irc::proto::{self, command};
use irc::{codec, connection, Connection};
use tokio::time::{self, Instant, Interval};

use crate::client::Client;
use crate::server::Server;
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
    },
    Disconnected {
        server: Server,
        is_initial: bool,
        error: Option<String>,
    },
    ConnectionFailed {
        server: Server,
        error: String,
    },
    MessagesReceived(Server, Vec<message::Encoded>),
}

enum State {
    Disconnected { last_retry: Option<Instant> },
    Connected { stream: Stream, batch: Batch },
}

enum Input {
    IrcMessage(Result<codec::ParseResult, codec::Error>),
    Batch(Vec<message::Encoded>),
    Send(proto::Message),
}

struct Stream {
    connection: Connection,
    receiver: mpsc::Receiver<proto::Message>,
}

pub async fn run(server: server::Entry, mut sender: mpsc::Sender<Update>) -> Never {
    const RECONNECT_DELAY: Duration = Duration::from_secs(10);

    let server::Entry { server, config } = server;

    let mut is_initial = true;
    let mut state = State::Disconnected { last_retry: None };
    // Notify app of initial disconnected state
    let _ = sender
        .send(Update::Disconnected {
            server: server.clone(),
            is_initial,
            error: None,
        })
        .await;

    loop {
        match &mut state {
            State::Disconnected { last_retry } => {
                if let Some(last_retry) = last_retry.as_ref() {
                    let remaining = RECONNECT_DELAY.saturating_sub(last_retry.elapsed());

                    if !remaining.is_zero() {
                        time::sleep(remaining).await;
                    }
                }

                match connect(config.clone()).await {
                    Ok((stream, client)) => {
                        log::info!("[{server}] connected");

                        let _ = sender
                            .send(Update::Connected {
                                server: server.clone(),
                                client,
                                is_initial,
                            })
                            .await;

                        is_initial = false;

                        state = State::Connected {
                            stream,
                            batch: Batch::new(),
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
                            })
                            .await;

                        *last_retry = Some(Instant::now());
                    }
                }
            }
            State::Connected { stream, batch } => {
                let input = stream::select(
                    stream::select(
                        (&mut stream.connection).map(Input::IrcMessage),
                        batch.map(Input::Batch),
                    ),
                    (&mut stream.receiver).map(Input::Send),
                )
                .next()
                .await
                .expect("stream input");

                match input {
                    Input::IrcMessage(Ok(Ok(message))) => {
                        batch.messages.push(message.into());
                    }
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
                }
            }
        }
    }
}

async fn connect(config: config::Server) -> Result<(Stream, Client), connection::Error> {
    let mut connection = Connection::new(&config.server, config.port, config.use_tls).await?;

    // Begin registration
    connection.send(command!("CAP", "LS", "302")).await?;

    // Identify
    {
        let nick = &config.nickname;
        let user = config.username.as_ref().unwrap_or(nick);
        let real = config.realname.as_ref().unwrap_or(nick);

        if let Some(pass) = config.password.as_ref() {
            connection.send(command!("PASS", pass)).await?;
        }
        connection.send(command!("NICK", nick)).await?;
        connection.send(command!("USER", user, real)).await?;
    }

    // Negotiate capbilities
    {
        let mut str_caps = String::new();
        let mut caps = vec![];

        while let Some(Ok(Ok(message))) = connection.next().await {
            log::trace!("Message received => {:?}", message);

            if let proto::Command::CAP(_, sub, a, b) = message.command {
                if sub.as_str() == "LS" {
                    let (cap_str, asterisk) = match (a, b) {
                        (Some(cap_str), None) => (cap_str, None),
                        (Some(asterisk), Some(cap_str)) => (cap_str, Some(asterisk)),
                        // Unreachable?
                        (None, None) | (None, Some(_)) => break,
                    };

                    str_caps = format!("{str_caps} {cap_str}");

                    if asterisk.is_none() {
                        break;
                    }
                }
            }
        }

        let server_caps = str_caps.split(' ').collect::<Vec<_>>();

        if server_caps.contains(&"server-time") {
            caps.push("server-time");
        }
        if server_caps.contains(&"batch") {
            caps.push("batch");
        }
        if server_caps.contains(&"labeled-response") {
            caps.push("labeled-response");

            // We require labeled-response so we can properly tag echo-messages
            if server_caps.contains(&"echo-message") {
                caps.push("echo-message");
            }
        }

        let caps = caps.join(" ");

        connection.send(command!("CAP", "REQ", caps)).await?;
    }

    // Finish
    connection.send(command!("CAP", "END")).await?;

    let (sender, receiver) = mpsc::channel(100);

    Ok((
        Stream {
            connection,
            receiver,
        },
        Client::new(config, sender),
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
