use std::collections::BTreeMap;
use std::io;
use std::net::IpAddr;
use std::path::PathBuf;
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, Utc};
use futures::channel::mpsc;
use futures::never::Never;
use futures::{FutureExt, SinkExt, StreamExt, future, stream};
use hashbrown::HashSet;
use irc::proto::{self, Command, command};
use irc::{CodecLog, Connection, codec, connection};
use tokio::fs::{self, File};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::time::{self, Instant, Interval, MissedTickBehavior};

use crate::client::Client;
use crate::config::server::IrcProtocolLogFormat;
use crate::server::Server;
use crate::time::Posix;
use crate::{config, environment, message, server};

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
        autoconnect: bool,
    },
    ConnectionFailed {
        server: Server,
        error: String,
        sent_time: DateTime<Utc>,
        autoconnect: bool,
    },
    MessagesReceived(Server, Vec<message::Encoded>),
    Remove(Server),
    UpdateConfiguration {
        server: Server,
        updated_config: Arc<config::Server>,
    },
    AutoconnectDisabled {
        server: Server,
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
        network_probe: bool,
        websocket_ping_time: Option<Interval>,
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
    WebSocketPing,
    Control(Control),
}

pub enum Control {
    Disconnect {
        error: Option<String>,
        disable_autoconnect: bool,
    },
    AuthenticationFailed {
        error: Option<String>,
    },
    Connect(bool),
    DisableAutoconnect,
    NetworkChanged {
        available: bool,
        addresses: Arc<HashSet<IpAddr>>,
    },
    Resumed,
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

    let (controller, control) = mpsc::channel(20);
    let mut control = control.peekable();

    let _ = sender.unbounded_send(Update::Controller {
        server: server.clone(),
        controller,
    });

    let mut is_initial = true;

    // Needs to be tracked across states as connection can fail during
    // authentication
    let mut connection_attempt = 0;

    let mut attempts_exhausted = false;
    let mut network_available = true;
    let mut last_connected: Option<Instant> = None;

    let mut state = State::Disconnected {
        autoconnect: config.autoconnect,
        retry: interval_at(Duration::ZERO, config.reconnect_delay),
    };

    // Notify app of initial disconnected state
    let _ = sender.unbounded_send(Update::Disconnected {
        server: server.clone(),
        is_initial,
        error: None,
        sent_time: Utc::now(),
        autoconnect: config.autoconnect,
    });

    loop {
        match &mut state {
            State::Disconnected { autoconnect, retry } => {
                let selection = {
                    if *autoconnect
                        && (network_available
                            // loopback servers stay reachable while the network is down
                            || is_loopback(&config, default_proxy.as_ref()))
                    {
                        stream::select(
                            (&mut control).boxed(),
                            retry
                                .tick()
                                .into_stream()
                                .map(|_| Control::Connect(true))
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
                        attempts_exhausted = false;

                        state = State::Disconnected {
                            autoconnect: config.autoconnect,
                            retry: interval_at(
                                Duration::ZERO,
                                config.reconnect_delay,
                            ),
                        };
                    }
                    Some(Control::Disconnect {
                        disable_autoconnect,
                        ..
                    }) => {
                        if disable_autoconnect {
                            *autoconnect = false;
                            attempts_exhausted = false;
                        }
                    }
                    Some(Control::NetworkChanged { available, .. }) => {
                        network_available = available;

                        if available && (*autoconnect || attempts_exhausted) {
                            log::info!(
                                "[{server}] network changed, reconnecting"
                            );

                            *autoconnect = true;
                            attempts_exhausted = false;
                            connection_attempt = 0;
                            *retry = network_retry(
                                last_connected,
                                config.reconnect_delay,
                            );
                        }
                    }
                    Some(Control::DisableAutoconnect) => {
                        *autoconnect = false;
                        attempts_exhausted = false;

                        let _ = sender.unbounded_send(
                            Update::AutoconnectDisabled {
                                server: server.clone(),
                            },
                        );
                    }
                    Some(Control::Connect(automated)) => {
                        if automated {
                            connection_attempt += 1;
                        } else {
                            *autoconnect = config.autoconnect;
                            connection_attempt = 1;
                            attempts_exhausted = false;
                        }

                        let _ = sender.unbounded_send(Update::Connecting {
                            server: server.clone(),
                            sent_time: Utc::now(),
                        });

                        let attempt = pin!(connect(
                            server.clone(),
                            config.clone(),
                            config
                                .proxy
                                .as_ref()
                                .or(default_proxy.as_ref())
                                .cloned(),
                        ));

                        // a network change aborts an attempt that may be stuck
                        // on the old network. peeking leaves the change queued,
                        // so it still resets retry on the next loop
                        let network_changed = pin!(async {
                            match Pin::new(&mut control).peek().await {
                                Some(Control::NetworkChanged { .. }) => (),
                                _ => future::pending().await,
                            }
                        });

                        let result = match future::select(
                            attempt,
                            network_changed,
                        )
                        .await
                        {
                            future::Either::Left((result, _)) => result
                                .map_err(|e| match e {
                                    connection::Error::Tls(e) => {
                                        format!("a TLS error occurred: {e}")
                                    }
                                    _ => e.to_string(),
                                }),
                            future::Either::Right(((), _)) => {
                                Err("network changed".to_string())
                            }
                        };

                        match result {
                            Ok((stream, client)) => {
                                log::info!("[{server}] connected");

                                last_connected = Some(Instant::now());

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
                                    network_probe: false,
                                    ping_time: interval_at(
                                        config.ping_time,
                                        config.ping_time,
                                    ),
                                    websocket_ping_time: config
                                        .use_websocket
                                        .then(|| {
                                            interval_at(
                                                config.websocket_ping_time,
                                                config.websocket_ping_time,
                                            )
                                        }),
                                    quit_requested: None,
                                };
                            }
                            Err(error) => {
                                log::info!(
                                    "[{server}] connection failed: {error}"
                                );

                                retry.reset();

                                if config.max_connection_attempts.is_some_and(
                                    |max_attempts| {
                                        connection_attempt >= max_attempts
                                    },
                                ) {
                                    attempts_exhausted = *autoconnect;
                                    *autoconnect = false;
                                }

                                let _ = sender.unbounded_send(
                                    Update::ConnectionFailed {
                                        server: server.clone(),
                                        error,
                                        sent_time: Utc::now(),
                                        autoconnect: *autoconnect,
                                    },
                                );
                            }
                        }
                    }
                    Some(Control::End(_)) => {
                        state = State::End;
                    }
                    Some(
                        Control::AuthenticationFailed { .. } | Control::Resumed,
                    )
                    | None => (),
                }
            }
            State::Connected {
                stream,
                batch,
                ping_time,
                ping_timeout,
                network_probe,
                websocket_ping_time,
                quit_requested,
            } => {
                let input = {
                    let mut select = stream::select_all([
                        (&mut stream.connection)
                            .map(Input::IrcMessage)
                            .chain(stream::once(async {
                                Input::IrcMessage(Err(io::Error::new(
                                    io::ErrorKind::UnexpectedEof,
                                    "connection closed",
                                )
                                .into()))
                            }))
                            .boxed(),
                        (&mut stream.receiver).map(Input::Send).boxed(),
                        ping_time
                            .tick()
                            .into_stream()
                            .map(|_| Input::Ping)
                            .boxed(),
                        batch.map(Input::Batch).boxed(),
                        (&mut control).map(Input::Control).boxed(),
                    ]);

                    if let Some(ws_ping) = websocket_ping_time.as_mut() {
                        select.push(
                            ws_ping
                                .tick()
                                .into_stream()
                                .map(|_| Input::WebSocketPing)
                                .boxed(),
                        );
                    }

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
                            .map(|()| {
                                Input::Control(Control::Disconnect {
                                    error: None,
                                    disable_autoconnect: true,
                                })
                            })
                            .boxed(),
                        );
                    }

                    select.next().await.expect("Connected select")
                };

                // we know the connection is alive, so we can defer the next ping,
                // except during a network probe, where buffered data proves nothing
                if matches!(input, Input::IrcMessage(Ok(_))) && !*network_probe
                {
                    ping_time.reset();
                }

                let autoconnect = quit_requested.is_none();
                let mut connection_error: Option<String> = None;
                let mut network_lost = false;

                match input {
                    Input::IrcMessage(Ok(Ok(message))) => match message.command
                    {
                        proto::Command::PING(token) => {
                            if let Err(e) = stream
                                .connection
                                .send(command!("PONG", token))
                                .await
                            {
                                connection_error = Some(e.to_string());
                            }
                        }
                        proto::Command::PONG(_, token) => {
                            let token = token.unwrap_or_default();
                            log::trace!("[{server}] pong received: {token}");

                            *ping_timeout = None;
                            *network_probe = false;
                        }
                        proto::Command::ERROR(error) => {
                            // If QUIT was requested, then ERROR is
                            // a valid acknowledgement
                            // https://modern.ircdocs.horse/#quit-message
                            connection_error = Some(error);
                        }
                        _ => {
                            batch.messages.push(message.into());
                        }
                    },
                    Input::IrcMessage(Ok(Err(e))) => {
                        log::warn!("message decoding failed: {e}");
                    }
                    Input::IrcMessage(Err(e)) => {
                        connection_error = Some(e.to_string());
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
                        } else if let Err(e) =
                            stream.connection.send(message).await
                        {
                            connection_error = Some(e.to_string());
                        }
                    }
                    Input::Ping => {
                        let now = Posix::now().as_nanos().to_string();
                        log::trace!("[{server}] ping sent: {now}");

                        if let Err(e) =
                            stream.connection.send(command!("PING", now)).await
                        {
                            connection_error = Some(e.to_string());
                        }

                        if ping_timeout.is_none() {
                            *ping_timeout = Some(interval_at(
                                config.ping_timeout,
                                config.ping_timeout,
                            ));
                        }
                    }
                    Input::PingTimeout => {
                        connection_error = Some("ping timeout".into());

                        // a network change most likely broke the connection,
                        // so reconnect now instead of after reconnect_delay
                        if *network_probe {
                            network_lost = true;
                        }
                    }
                    Input::WebSocketPing => {
                        let now = Posix::now().as_nanos().to_string();
                        log::trace!("[{server}] websocket ping sent: {now}");

                        match stream
                            .connection
                            .send_ws_ping(config.ping_timeout)
                            .await
                        {
                            Ok(()) => {}
                            Err(e) => {
                                log::error!(
                                    "[{server}] failed to send websocket ping: {e}"
                                );

                                connection_error = Some(e.to_string());
                            }
                        }
                    }
                    Input::Control(control) => match control {
                        Control::UpdateConfiguration(
                            updated_config,
                            updated_default_proxy,
                        ) => {
                            // If connection detail(s) change, then disconnect
                            if updated_config.has_same_connection_settings(
                                updated_default_proxy.as_ref(),
                                &config,
                                default_proxy.as_ref(),
                            ) {
                                connection_attempt = 0;
                                let _ = sender.unbounded_send(
                                    Update::Disconnected {
                                        server: server.clone(),
                                        is_initial,
                                        error: None,
                                        sent_time: Utc::now(),
                                        autoconnect: updated_config.autoconnect,
                                    },
                                );
                                state = State::Disconnected {
                                    autoconnect: updated_config.autoconnect,
                                    retry: interval_at(
                                        Duration::from_secs(1),
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
                        Control::Connect(_) | Control::DisableAutoconnect => (),
                        Control::NetworkChanged {
                            available,
                            addresses,
                        } => {
                            network_available = available;

                            if stream
                                .connection
                                .local_ip()
                                .is_some_and(|ip| !addresses.contains(&ip))
                            {
                                connection_error =
                                    Some("network changed".into());
                                network_lost = true;
                            } else {
                                // ping now to find out if the connection survived
                                *network_probe = true;
                                ping_time.reset_immediately();
                            }
                        }
                        Control::Resumed => {
                            // timers pause during sleep
                            *network_probe = true;
                            ping_time.reset_immediately();
                        }
                        Control::AuthenticationFailed { error } => {
                            let autoconnect = config.autoconnect
                                && config.max_connection_attempts.is_none_or(
                                    |max_attempt| {
                                        connection_attempt < max_attempt
                                    },
                                );

                            let _ =
                                sender.unbounded_send(Update::Disconnected {
                                    server: server.clone(),
                                    is_initial,
                                    error,
                                    sent_time: Utc::now(),
                                    autoconnect,
                                });
                            state = State::Disconnected {
                                autoconnect,
                                retry: interval_at(
                                    config.reconnect_delay,
                                    config.reconnect_delay,
                                ),
                            };
                        }
                        Control::Disconnect {
                            error,
                            disable_autoconnect,
                        } => {
                            let autoconnect = if disable_autoconnect {
                                false
                            } else {
                                config.autoconnect
                            };
                            connection_attempt = 0;

                            let _ =
                                sender.unbounded_send(Update::Disconnected {
                                    server: server.clone(),
                                    is_initial,
                                    error,
                                    sent_time: Utc::now(),
                                    autoconnect,
                                });
                            state = State::Disconnected {
                                autoconnect,
                                retry: interval_at(
                                    config.reconnect_delay,
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

                if let Some(error) = connection_error {
                    log::info!("[{server}] disconnected: {error}");

                    // flush batched messages so they are processed before
                    // the client state changes to disconnected
                    if let State::Connected { batch, .. } = &mut state
                        && !batch.messages.is_empty()
                    {
                        let _ =
                            sender.unbounded_send(Update::MessagesReceived(
                                server.clone(),
                                std::mem::take(&mut batch.messages),
                            ));
                    }

                    connection_attempt = 0;

                    // autoconnect is off only after QUIT, where the server
                    // closing the connection is expected
                    let _ = sender.unbounded_send(Update::Disconnected {
                        server: server.clone(),
                        is_initial,
                        error: autoconnect.then_some(error),
                        sent_time: Utc::now(),
                        autoconnect,
                    });
                    state = State::Disconnected {
                        autoconnect,
                        retry: if network_lost {
                            network_retry(
                                last_connected,
                                config.reconnect_delay,
                            )
                        } else {
                            interval_at(
                                config.reconnect_delay,
                                config.reconnect_delay,
                            )
                        },
                    };
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
    let logger = if config.irc_protocol_log.enabled {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        tokio::task::spawn(log_codec(
            server.clone(),
            config.irc_protocol_log,
            receiver,
        ));

        Some(sender)
    } else {
        None
    };

    let connection =
        Connection::new(config.connection(proxy), irc::Codec::new(logger))
            .await?;

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

async fn log_codec(
    server: Server,
    config: config::server::IrcProtocolLog,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<CodecLog>,
) {
    let log_writer = LogWriter::new(&server).await;

    match log_writer {
        Ok(mut log_writer) => {
            let mut set_timeout_to_flush = false;

            while let Some(action) = if set_timeout_to_flush {
                tokio::time::timeout(
                    Duration::from_millis(500),
                    receiver.recv(),
                )
                .await
                .transpose()
            } else {
                receiver.recv().await.map(Ok)
            } {
                match action {
                    Ok(message) => {
                        log_writer.write(message, &config).await;
                        set_timeout_to_flush = true;
                    }
                    Err(_) => {
                        log_writer.flush().await;
                        set_timeout_to_flush = false;
                    }
                }
            }

            log_writer.shutdown().await;
        }
        Err(error) => {
            log::error!("unable to create log writer for {server}: {error}");
        }
    }
}

struct LogWriter {
    log_dir: PathBuf,
    date_writer: Option<(NaiveDate, BufWriter<File>)>,
}

impl LogWriter {
    pub async fn new(server: &Server) -> Result<Self, std::io::Error> {
        let data_dir = environment::data_dir();

        let mut log_dir = data_dir
            .join("irc_protocol_logs")
            .join(sanitize_filename::sanitize(server.name.as_ref()));

        if let Some(network) = &server.network {
            log_dir = log_dir.join(sanitize_filename::sanitize(&network.name));
        };

        if !log_dir.exists() {
            fs::create_dir_all(&log_dir).await?;
        }

        Ok(Self {
            log_dir,
            date_writer: None,
        })
    }

    pub async fn write(
        &mut self,
        message: CodecLog,
        config: &config::server::IrcProtocolLog,
    ) {
        let now = Local::now();

        let today = match config.timestamp {
            config::logs::Timestamp::Local => now.date_naive(),
            config::logs::Timestamp::Utc => now.to_utc().date_naive(),
        };

        if let Some((date, writer)) = &mut self.date_writer {
            if today != *date {
                let _ = writer.flush().await;

                self.create_date_writer(today).await;
            }
        } else {
            self.create_date_writer(today).await;
        }

        if let Some((_, writer)) = &mut self.date_writer {
            let _ = writer
                .write_all(LogWriter::format(message, now, config).as_bytes())
                .await;
        }
    }

    async fn create_date_writer(&mut self, date: NaiveDate) {
        let log_file = date.format("%Y-%m-%d.log").to_string();

        self.date_writer = File::options()
            .append(true)
            .create(true)
            .open(self.log_dir.join(log_file))
            .await
            .ok()
            .map(|writer| (date, BufWriter::new(writer)));
    }

    fn format(
        message: CodecLog,
        received_at: DateTime<Local>,
        config: &config::server::IrcProtocolLog,
    ) -> String {
        let received_at_format = "%H:%M:%S%.3f";
        let received_at = match config.timestamp {
            config::logs::Timestamp::Local => {
                received_at.format(received_at_format)
            }
            config::logs::Timestamp::Utc => {
                received_at.to_utc().format(received_at_format)
            }
        };

        let (direction, divider, message_content) = match config.format {
            IrcProtocolLogFormat::Halloy => match message {
                CodecLog::Received(content) => ("RECEIVED", " -- ", content),
                CodecLog::Sent(content) => ("  SENT  ", " -- ", content),
            },
            IrcProtocolLogFormat::Goguma => match message {
                CodecLog::Received(content) => ("<-", " ", content),
                CodecLog::Sent(content) => ("->", " ", content),
            },
        };

        format!("{received_at} {direction}{divider}{message_content}")
    }

    pub async fn flush(&mut self) {
        if let Some((_, writer)) = &mut self.date_writer {
            let _ = writer.flush().await;
        }
    }

    pub async fn shutdown(&mut self) {
        if let Some((_, writer)) = &mut self.date_writer {
            let _ = writer.flush().await;

            let _ = writer.shutdown().await;
        }
    }
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

fn is_loopback(
    config: &config::Server,
    default_proxy: Option<&config::Proxy>,
) -> bool {
    config.proxy.is_none()
        && default_proxy.is_none()
        && (config.server.eq_ignore_ascii_case("localhost")
            || config
                .server
                .parse::<IpAddr>()
                .is_ok_and(|ip| ip.is_loopback()))
}

fn network_retry(
    last_connected: Option<Instant>,
    reconnect_delay: Duration,
) -> Interval {
    // delay reconnects that follow a recent successful connection
    let delay = last_connected.map_or(Duration::ZERO, |at| {
        reconnect_delay.saturating_sub(at.elapsed())
    });

    interval_at(delay, reconnect_delay)
}

fn interval_at(delay: Duration, period: Duration) -> Interval {
    let mut interval = time::interval_at(Instant::now() + delay, period);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    interval
}

#[derive(Debug, Default)]
pub struct Map {
    controllers: BTreeMap<Server, mpsc::Sender<Control>>,
    offline: bool,
}

impl Map {
    pub fn insert(
        &mut self,
        server: Server,
        mut controller: mpsc::Sender<Control>,
    ) {
        if self.offline {
            let _ = controller.try_send(Control::NetworkChanged {
                available: false,
                addresses: Arc::default(),
            });
        }

        self.controllers.insert(server, controller);
    }

    pub fn update_config(
        &mut self,
        server: &Server,
        config: Arc<config::Server>,
        default_proxy: Option<config::Proxy>,
    ) {
        if let Some(controller) = self.controllers.get_mut(server) {
            let _ = controller
                .try_send(Control::UpdateConfiguration(config, default_proxy));
        }
    }

    pub fn authentication_failed(
        &mut self,
        server: &Server,
        error: Option<String>,
    ) {
        if let Some(controller) = self.controllers.get_mut(server) {
            let _ =
                controller.try_send(Control::AuthenticationFailed { error });
        }
    }

    pub fn connect(&mut self, server: &Server) {
        if let Some(controller) = self.controllers.get_mut(server) {
            let _ = controller.try_send(Control::Connect(false));
        }
    }

    pub fn disable_autoconnect(&mut self, server: &Server) {
        if let Some(controller) = self.controllers.get_mut(server) {
            let _ = controller.try_send(Control::DisableAutoconnect);
        }
    }

    pub fn network_changed(
        &mut self,
        available: bool,
        addresses: &Arc<HashSet<IpAddr>>,
    ) {
        self.offline = !available;

        for controller in self.controllers.values_mut() {
            let _ = controller.try_send(Control::NetworkChanged {
                available,
                addresses: addresses.clone(),
            });
        }
    }

    pub fn resumed(&mut self) {
        for controller in self.controllers.values_mut() {
            let _ = controller.try_send(Control::Resumed);
        }
    }

    pub fn remove(&mut self, server: &Server) {
        self.controllers.remove(server);
    }

    pub fn end(&mut self, server: &Server, reason: &Option<String>) {
        if let Some(controller) = self.controllers.get_mut(server) {
            let _ = controller.try_send(Control::End(reason.clone()));
        }

        self.controllers.remove(server);
    }

    pub fn connected(&self) -> HashSet<Server> {
        self.controllers.keys().cloned().collect()
    }

    pub fn exit(&mut self, reason: &Option<String>) {
        for controller in self.controllers.values_mut() {
            let _ = controller.try_send(Control::End(reason.clone()));
        }
    }
}
