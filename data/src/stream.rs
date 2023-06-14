use std::time::Duration;

use futures::stream::{self, BoxStream};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::{self, Instant, Interval};

use crate::client::Connection;
use crate::server;
use crate::server::Server;

pub type Result<T = Event, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Connection(irc::error::Error),
}

#[derive(Debug)]
pub enum Event {
    Ready(mpsc::Sender<Message>),
    Connected(Server, Connection),
    MessagesReceived(Vec<(Server, irc::proto::Message)>),
}

#[derive(Debug, Clone)]
pub enum Message {
    Connect(String, server::Config),
}

enum State {
    Disconnected,
    Ready {
        receiver: mpsc::Receiver<Message>,
    },
    Connected {
        batch: Batch,
        receiver: mpsc::Receiver<Message>,
        servers: Vec<ServerData>,
    },
}

struct ServerData {
    name: String,
    config: server::Config,
    stream: irc::client::ClientStream,
}
enum Input {
    Message(Option<Message>),
    IrcMessage(usize, Result<irc::proto::Message, irc::error::Error>),
    Batch(Vec<(Server, irc::proto::Message)>),
}

pub fn run() -> BoxStream<'static, Result> {
    stream::unfold(State::Disconnected, move |state| async move {
        match state {
            State::Disconnected => {
                let (sender, receiver) = mpsc::channel(20);

                Some((Ok(Event::Ready(sender)), State::Ready { receiver }))
            }
            State::Ready { mut receiver } => loop {
                if let Some(Message::Connect(name, config)) = receiver.recv().await {
                    match connect(config.clone()).await {
                        Ok((stream, connection)) => {
                            let servers = vec![ServerData {
                                name: name.clone(),
                                config: config.clone(),
                                stream,
                            }];
                            let server =
                                Server::new(name, config.server.as_ref().expect("server hostname"));

                            return Some((
                                Ok(Event::Connected(server, connection)),
                                State::Connected {
                                    batch: Batch::new(),
                                    receiver,
                                    servers,
                                },
                            ));
                        }
                        Err(e) => {
                            return Some((Err(Error::Connection(e)), State::Ready { receiver }));
                        }
                    }
                }
            },
            State::Connected {
                mut batch,
                mut receiver,
                mut servers,
            } => loop {
                let input = {
                    let mut select = stream::select(
                        stream::select(
                            stream::select_all(servers.iter_mut().enumerate().map(
                                |(idx, server)| {
                                    (&mut server.stream)
                                        .map(move |result| Input::IrcMessage(idx, result))
                                },
                            )),
                            receiver.recv().map(Input::Message).into_stream().boxed(),
                        ),
                        (&mut batch).map(Input::Batch),
                    );

                    select.next().await.expect("Await stream input")
                };

                match input {
                    Input::Message(Some(message)) => match message {
                        Message::Connect(name, config) => match connect(config.clone()).await {
                            Ok((stream, connection)) => {
                                servers.push(ServerData {
                                    name: name.clone(),
                                    config: config.clone(),
                                    stream,
                                });
                                let server = Server::new(
                                    name,
                                    config.server.as_ref().expect("server hostname"),
                                );

                                return Some((
                                    Ok(Event::Connected(server, connection)),
                                    State::Connected {
                                        batch,
                                        receiver,
                                        servers,
                                    },
                                ));
                            }
                            Err(e) => {
                                return Some((
                                    Err(Error::Connection(e)),
                                    State::Ready { receiver },
                                ));
                            }
                        },
                    },
                    Input::IrcMessage(idx, Ok(message)) => {
                        let server = &servers[idx];
                        let server = Server::new(
                            &server.name,
                            server.config.server.as_ref().expect("server hostname"),
                        );
                        batch.messages.push((server, message));
                    }
                    Input::Message(None) => {}
                    Input::IrcMessage(_, Err(_)) => {} // TODO: Handle?
                    Input::Batch(messages) => {
                        return Some((
                            Ok(Event::MessagesReceived(messages)),
                            State::Connected {
                                batch,
                                receiver,
                                servers,
                            },
                        ));
                    }
                }
            },
        }
    })
    .boxed()
}

async fn connect(
    config: server::Config,
) -> Result<(irc::client::ClientStream, Connection), irc::error::Error> {
    let mut client = irc::client::Client::from_config((*config).clone()).await?;
    client.identify()?;

    Ok((client.stream()?, Connection::new(client)))
}

struct Batch {
    interval: Interval,
    messages: Vec<(Server, irc::proto::Message)>,
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
    type Item = Vec<(Server, irc::proto::Message)>;

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
