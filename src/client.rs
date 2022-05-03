// TODO: This should live in its own crate.

use data::client::Sender;
use data::server::Server;
use futures::FutureExt;
use iced::futures::stream::{self, BoxStream, StreamExt};
use iced::Subscription;
use iced_native::subscription::Recipe;
use iced_native::Hasher;
use tokio::sync::mpsc;

pub fn run() -> Subscription<Result> {
    Subscription::from_recipe(Client {})
}

pub type Result<T = Event, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Connection(irc::error::Error),
}

#[derive(Debug)]
pub enum Event {
    Ready(mpsc::Sender<Message>),
    Connected(Server, Sender),
    MessageReceived(Server, data::message::Message),
}

#[derive(Debug, Clone)]
pub enum Message {
    Connect(irc::client::data::Config),
}

enum State {
    Disconnected,
    Ready {
        receiver: mpsc::Receiver<Message>,
    },
    Connected {
        receiver: mpsc::Receiver<Message>,
        servers: Vec<ServerData>,
    },
}

struct ServerData {
    config: irc::client::data::Config,
    stream: irc::client::ClientStream,
}

enum Input {
    Message(Option<Message>),
    IrcMessage(usize, Result<irc::proto::Message, irc::error::Error>), // TODO: We probably need to encode some "Server Name" to proporly map response to the right server
}

pub struct Client {}

impl<E> Recipe<iced_native::Hasher, E> for Client {
    type Output = Result;

    fn hash(&self, state: &mut Hasher) {
        use std::hash::Hash;

        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<E>) -> BoxStream<Self::Output> {
        stream::unfold(State::Disconnected, move |state| async move {
            match state {
                State::Disconnected => {
                    let (sender, receiver) = mpsc::channel(20);

                    Some((Ok(Event::Ready(sender)), State::Ready { receiver }))
                }
                State::Ready { mut receiver } => loop {
                    if let Some(Message::Connect(config)) = receiver.recv().await {
                        match connect(config.clone()).await {
                            Ok((stream, sender)) => {
                                let servers = vec![ServerData {
                                    config: config.clone(),
                                    stream,
                                }];
                                let server = config.server.expect("expected server").into();

                                return Some((
                                    Ok(Event::Connected(server, sender)),
                                    State::Connected { receiver, servers },
                                ));
                            }
                            Err(e) => {
                                return Some((
                                    Err(Error::Connection(e)),
                                    State::Ready { receiver },
                                ));
                            }
                        }
                    }
                },
                State::Connected {
                    mut receiver,
                    mut servers,
                } => loop {
                    let input = {
                        let mut select = stream::select(
                            stream::select_all(servers.iter_mut().map(|server| &mut server.stream))
                                .enumerate()
                                .map(|(idx, result)| Input::IrcMessage(idx, result)),
                            receiver.recv().map(Input::Message).into_stream().boxed(),
                        );

                        select.next().await.expect("Await stream input")
                    };

                    match input {
                        Input::Message(Some(message)) => match message {
                            Message::Connect(config) => match connect(config.clone()).await {
                                Ok((stream, sender)) => {
                                    servers.push(ServerData {
                                        config: config.clone(),
                                        stream,
                                    });
                                    let server = config.server.expect("expected server").into();

                                    return Some((
                                        Ok(Event::Connected(server, sender)),
                                        State::Connected { receiver, servers },
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
                            // let sender = &senders[idx];
                            // process_msg(sender, message)?; // TODO: ??

                            println!("{:?}", message.command);

                            return Some((
                                Ok(Event::MessageReceived(
                                    server
                                        .config
                                        .server
                                        .clone()
                                        .expect("expected server")
                                        .into(),
                                    message.into(),
                                )),
                                State::Connected { receiver, servers },
                            ));
                        }
                        Input::Message(None) => {}
                        Input::IrcMessage(_, Err(_)) => {} // TODO: Handle?
                    }
                },
            }
        })
        .boxed()
    }
}

async fn connect(
    config: irc::client::data::Config,
) -> Result<(irc::client::ClientStream, Sender), irc::error::Error> {
    let mut client = irc::client::Client::from_config(config).await?;
    client.identify()?;

    Ok((client.stream()?, client.sender().into()))
}
