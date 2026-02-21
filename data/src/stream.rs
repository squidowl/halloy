use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use futures::channel::mpsc;
use futures::never::Never;
use futures::{FutureExt, SinkExt, StreamExt, future, stream};
use irc::proto::{self, Command, command};
use irc::{Connection, codec, connection};
use tokio::time::{self, Instant, Interval};

use crate::client::Client;
use crate::server::Server;
use crate::time::Posix;
use crate::{config, message, server};

const QUIT_REQUEST_TIMEOUT: Duration = Duration::from_millis(400);

pub type Result<T = Update, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Connection(connection::Error),
}

#[derive(Debug)]
pub enum Update {
    Controller {
        server: Server,
        controller: mpsc::Sender<Control>,
    },
    Connecting {
        server: Server,
        sent_time: DateTime<Utc>,
    },
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
    Remove(Server),
    UpdateConfiguration {
        server: Server,
        updated_config: Arc<config::Server>,
    },
}

enum State {
    Disconnected {
        autoconnect: bool,
        retry: Interval,
    },
    Connected {
        stream: Stream,
        batch: Batch,
        ping_time: Interval,
        ping_timeout: Option<Interval>,
        quit_requested: Option<Instant>,
    },
    End,
}

enum Input {
    IrcMessage(Result<codec::ParseResult, codec::Error>),
    Batch(Vec<message::Encoded>),
    Send(proto::Message),
    Ping,
    PingTimeout,
    Control(Control),
}

pub enum Control {
    Disconnect(Option<String>),
    Connect,
    End(Option<String>),
    UpdateConfiguration(Arc<config::Server>, Option<config::Proxy>),
}

struct Stream {
    connection: Connection<irc::Codec>,
    receiver: mpsc::Receiver<proto::Message>,
}

pub fn run(
    server: server::Entry,
    proxy: Option<config::Proxy>,
) -> impl futures::Stream<Item = Update> {
    let (sender, receiver) = mpsc::unbounded();

    // Spawn to unblock backend from iced stream which has backpressure
    let runner =
        stream::once(async { tokio::spawn(_run(server, proxy, sender)).await })
            .map(|_| unreachable!());

    stream::select(receiver, runner)
}

async fn _run(
    server: server::Entry,
    mut default_proxy: Option<config::Proxy>,
    sender: mpsc::UnboundedSender<Update>,
) -> Never {
    let server::Entry { server, mut config } = server;

    let (controller, mut control) = mpsc::channel(20);

    let _ = sender.unbounded_send(Update::Controller {
        server: server.clone(),
        controller,
    });

    let mut is_initial = true;
    let mut state = State::Disconnected {
        autoconnect: config.autoconnect,
        retry: time::interval(config.reconnect_delay),
    };

    // Notify app of initial disconnected state
    let _ = sender.unbounded_send(Update::Disconnected {
        server: server.clone(),
        is_initial,
        error: None,
        sent_time: Utc::now(),
    });

    loop {
        match &mut state {
            State::Disconnected { autoconnect, retry } => {
                let selection = {
                    if *autoconnect {
                        stream::select(
                            (&mut control).boxed(),
                            retry
                                .tick()
                                .into_stream()
                                .map(|_| Control::Connect)
                                .boxed(),
                        )
                        .next()
                        .await
                    } else {
                        control.next().await
                    }
                };

                match selection {
                    Some(Control::UpdateConfiguration(
                        updated_config,
                        updated_default_proxy,
                    )) => {
                        config = updated_config;
                        default_proxy = updated_default_proxy;

                        state = State::Disconnected {
                            autoconnect: config.autoconnect,
                            retry: time::interval(config.reconnect_delay),
                        };
                    }
                    Some(Control::Disconnect(_)) => {
                        *autoconnect = false;
                    }
                    Some(Control::Connect) => {
                        let _ = sender.unbounded_send(Update::Connecting {
                            server: server.clone(),
                            sent_time: Utc::now(),
                        });

                        match connect(
                            server.clone(),
                            config.clone(),
                            config
                                .proxy
                                .as_ref()
                                .or(default_proxy.as_ref())
                                .cloned(),
                        )
                        .await
                        {
                            Ok((stream, client)) => {
                                log::info!("[{server}] connected");

                                let _ =
                                    sender.unbounded_send(Update::Connected {
                                        server: server.clone(),
                                        client,
                                        is_initial,
                                        sent_time: Utc::now(),
                                    });

                                is_initial = false;

                                state = State::Connected {
                                    stream,
                                    batch: Batch::new(),
                                    ping_timeout: None,
                                    ping_time: ping_time_interval(
                                        config.ping_time,
                                    ),
                                    quit_requested: None,
                                };
                            }
                            Err(e) => {
                                let error = match e {
                                    // unwrap Tls-specific error enums to access more error info
                                    connection::Error::Tls(e) => {
                                        format!("a TLS error occurred: {e}")
                                    }
                                    _ => e.to_string(),
                                };

                                log::info!(
                                    "[{server}] connection failed: {error}"
                                );

                                let _ = sender.unbounded_send(
                                    Update::ConnectionFailed {
                                        server: server.clone(),
                                        error,
                                        sent_time: Utc::now(),
                                    },
                                );

                                retry.reset();
                            }
                        }
                    }
                    Some(Control::End(_)) => {
                        state = State::End;
                    }
                    None => (),
                }
            }
            State::Connected {
                stream,
                batch,
                ping_time,
                ping_timeout,
                quit_requested,
            } => {
                let input = {
                    let mut select = stream::select_all([
                        (&mut stream.connection).map(Input::IrcMessage).boxed(),
                        (&mut stream.receiver).map(Input::Send).boxed(),
                        ping_time
                            .tick()
                            .into_stream()
                            .map(|_| Input::Ping)
                            .boxed(),
                        batch.map(Input::Batch).boxed(),
                        (&mut control).map(Input::Control).boxed(),
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

                    if let Some(requested_at) = quit_requested {
                        select.push(
                            time::sleep_until(
                                *requested_at + QUIT_REQUEST_TIMEOUT,
                            )
                            .into_stream()
                            .map(|()| Input::Control(Control::Disconnect(None)))
                            .boxed(),
                        );
                    }

                    select.next().await.expect("Connected select")
                };

                match input {
                    Input::IrcMessage(Ok(Ok(message))) => match message.command
                    {
                        proto::Command::PING(token) => {
                            let _ = stream
                                .connection
                                .send(command!("PONG", token))
                                .await;
                        }
                        proto::Command::PONG(_, token) => {
                            let token = token.unwrap_or_default();
                            log::trace!("[{server}] pong received: {token}");

                            *ping_timeout = None;
                        }
                        proto::Command::ERROR(error) => {
                            if quit_requested.is_some() {
                                let _ = sender.unbounded_send(
                                    Update::Disconnected {
                                        server: server.clone(),
                                        is_initial,
                                        error: None,
                                        sent_time: Utc::now(),
                                    },
                                );

                                // If QUIT was requested, then ERROR is
                                // a valid acknowledgement
                                // https://modern.ircdocs.horse/#quit-message
                                state = State::Disconnected {
                                    autoconnect: false,
                                    retry: time::interval_at(
                                        Instant::now() + config.reconnect_delay,
                                        config.reconnect_delay,
                                    ),
                                };
                            } else {
                                log::info!("[{server}] disconnected: {error}");
                                let _ = sender.unbounded_send(
                                    Update::Disconnected {
                                        server: server.clone(),
                                        is_initial,
                                        error: Some(error),
                                        sent_time: Utc::now(),
                                    },
                                );
                                state = State::Disconnected {
                                    autoconnect: true,
                                    retry: time::interval_at(
                                        Instant::now() + config.reconnect_delay,
                                        config.reconnect_delay,
                                    ),
                                };
                            }
                        }
                        _ => {
                            batch.messages.push(message.into());
                        }
                    },
                    Input::IrcMessage(Ok(Err(e))) => {
                        log::warn!("message decoding failed: {e}");
                    }
                    Input::IrcMessage(Err(e)) => {
                        log::info!("[{server}] disconnected: {e}");
                        let _ = sender.unbounded_send(Update::Disconnected {
                            server: server.clone(),
                            is_initial,
                            error: Some(e.to_string()),
                            sent_time: Utc::now(),
                        });
                        state = State::Disconnected {
                            autoconnect: true,
                            retry: time::interval_at(
                                Instant::now() + config.reconnect_delay,
                                config.reconnect_delay,
                            ),
                        };
                    }
                    Input::Batch(messages) => {
                        let _ = sender.unbounded_send(
                            Update::MessagesReceived(server.clone(), messages),
                        );
                    }
                    Input::Send(message) => {
                        log::trace!(
                            "[{server}] Sending message => {message:?}"
                        );

                        if let Command::QUIT(_) = &message.command {
                            let _ = stream.connection.send(message).await;

                            log::info!("[{server}] quit");

                            *quit_requested = Some(Instant::now());
                        } else {
                            let _ = stream.connection.send(message).await;
                        }
                    }
                    Input::Ping => {
                        let now = Posix::now().as_nanos().to_string();
                        log::trace!("[{server}] ping sent: {now}");

                        let _ =
                            stream.connection.send(command!("PING", now)).await;

                        if ping_timeout.is_none() {
                            *ping_timeout = Some(ping_timeout_interval(
                                config.ping_timeout,
                            ));
                        }
                    }
                    Input::PingTimeout => {
                        log::info!("[{server}] ping timeout");
                        let _ = sender.unbounded_send(Update::Disconnected {
                            server: server.clone(),
                            is_initial,
                            error: Some("ping timeout".into()),
                            sent_time: Utc::now(),
                        });
                        state = State::Disconnected {
                            autoconnect: true,
                            retry: time::interval_at(
                                Instant::now() + config.reconnect_delay,
                                config.reconnect_delay,
                            ),
                        };
                    }
                    Input::Control(control) => match control {
                        Control::UpdateConfiguration(
                            updated_config,
                            updated_default_proxy,
                        ) => {
                            // If connection detail(s) change, then disconnect
                            if config.server != updated_config.server
                                || config.port != updated_config.port
                                || config.use_tls != updated_config.use_tls
                                || config.dangerously_accept_invalid_certs
                                    != updated_config
                                        .dangerously_accept_invalid_certs
                                || config.root_cert_path
                                    != updated_config.root_cert_path
                                || config
                                    .proxy
                                    .as_ref()
                                    .or(default_proxy.as_ref())
                                    != updated_config
                                        .proxy
                                        .as_ref()
                                        .or(updated_default_proxy.as_ref())
                                || config.username != updated_config.username
                                || config.password != updated_config.password
                                || config.password_file
                                    != updated_config.password_file
                                || config.password_file_first_line_only
                                    != updated_config
                                        .password_file_first_line_only
                                || config.password_command
                                    != updated_config.password_command
                                || config.sasl != updated_config.sasl
                            {
                                let _ = sender.unbounded_send(
                                    Update::Disconnected {
                                        server: server.clone(),
                                        is_initial,
                                        error: None,
                                        sent_time: Utc::now(),
                                    },
                                );
                                state = State::Disconnected {
                                    autoconnect: updated_config.autoconnect,
                                    retry: time::interval_at(
                                        Instant::now() + Duration::from_secs(1),
                                        config.reconnect_delay,
                                    ),
                                };
                            } else {
                                let _ = sender.unbounded_send(
                                    Update::UpdateConfiguration {
                                        server: server.clone(),
                                        updated_config: updated_config.clone(),
                                    },
                                );
                            }

                            config = updated_config;
                            default_proxy = updated_default_proxy;
                        }
                        Control::Connect => (),
                        Control::Disconnect(error) => {
                            let _ =
                                sender.unbounded_send(Update::Disconnected {
                                    server: server.clone(),
                                    is_initial,
                                    error,
                                    sent_time: Utc::now(),
                                });
                            state = State::Disconnected {
                                autoconnect: false,
                                retry: time::interval_at(
                                    Instant::now() + config.reconnect_delay,
                                    config.reconnect_delay,
                                ),
                            };
                        }
                        Control::End(reason) => {
                            let _ = stream
                                .connection
                                .send(Command::QUIT(reason).into())
                                .await;

                            state = State::End;
                        }
                    },
                }
            }
            State::End => {
                let _ = sender.unbounded_send(Update::Remove(server.clone()));

                // Wait forever until this stream is dropped by the frontend
                future::pending::<()>().await;
            }
        }
    }
}

async fn connect(
    server: Server,
    config: Arc<config::Server>,
    proxy: Option<config::Proxy>,
) -> Result<(Stream, Client), connection::Error> {
    let connection =
        Connection::new(config.connection(proxy), irc::Codec).await?;

    let (sender, receiver) = mpsc::channel(100);

    let mut client = Client::new(server, config, sender);
    if let Err(e) = client.connect() {
        log::error!("Error when connecting client: {e:?}");
    }

    Ok((
        Stream {
            connection,
            receiver,
        },
        client,
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

#[derive(Debug, Default)]
pub struct Map(BTreeMap<Server, mpsc::Sender<Control>>);

impl Map {
    pub fn insert(
        &mut self,
        server: Server,
        controller: mpsc::Sender<Control>,
    ) {
        self.0.insert(server, controller);
    }

    pub fn update_config(
        &mut self,
        server: &Server,
        config: Arc<config::Server>,
        default_proxy: Option<config::Proxy>,
    ) {
        if let Some(controller) = self.0.get_mut(server) {
            let _ = controller
                .try_send(Control::UpdateConfiguration(config, default_proxy));
        }
    }

    pub fn disconnect(&mut self, server: &Server, error: Option<String>) {
        if let Some(controller) = self.0.get_mut(server) {
            let _ = controller.try_send(Control::Disconnect(error));
        }
    }

    pub fn connect(&mut self, server: &Server) {
        if let Some(controller) = self.0.get_mut(server) {
            let _ = controller.try_send(Control::Connect);
        }
    }

    pub fn remove(&mut self, server: &Server) {
        self.0.remove(server);
    }

    pub fn end(&mut self, server: &Server, reason: &Option<String>) {
        if let Some(controller) = self.0.get_mut(server) {
            let _ = controller.try_send(Control::End(reason.clone()));
        }

        self.0.remove(server);
    }

    pub fn exit(&mut self, reason: &Option<String>) -> HashSet<Server> {
        for controller in self.0.values_mut() {
            let _ = controller.try_send(Control::End(reason.clone()));
        }

        self.0.keys().cloned().collect()
    }
}
