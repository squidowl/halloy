use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_compression::tokio::write::GzipEncoder;
use chrono::{DateTime, Local, NaiveDate, Utc};
use futures::channel::mpsc;
use futures::never::Never;
use futures::{FutureExt, SinkExt, StreamExt, future, stream};
use irc::proto::{self, Command, command};
use irc::{CodecLog, Connection, codec, connection};
use tokio::fs::{self, File};
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};
use tokio::time::{self, Instant, Interval};

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
    Disconnect {
        error: Option<String>,
        disable_autoconnect: bool,
    },
    AuthenticationFailed {
        error: Option<String>,
    },
    Connect(bool),
    DisableAutoconnect,
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

    // Needs to be tracked across states as connection can fail during
    // authentication
    let mut connection_attempt = 0;

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
        autoconnect: config.autoconnect,
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

                        state = State::Disconnected {
                            autoconnect: config.autoconnect,
                            retry: time::interval(config.reconnect_delay),
                        };
                    }
                    Some(Control::Disconnect {
                        disable_autoconnect,
                        ..
                    }) => {
                        if disable_autoconnect {
                            *autoconnect = false;
                        }
                    }
                    Some(Control::DisableAutoconnect) => {
                        *autoconnect = false;

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
                        }

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

                                retry.reset();

                                if config.max_connection_attempts.is_some_and(
                                    |max_attempts| {
                                        connection_attempt >= max_attempts
                                    },
                                ) {
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
                    Some(Control::AuthenticationFailed { .. }) | None => (),
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
                            let autoconnect = !quit_requested.is_some();
                            connection_attempt = 0;

                            if quit_requested.is_some() {
                                let _ = sender.unbounded_send(
                                    Update::Disconnected {
                                        server: server.clone(),
                                        is_initial,
                                        error: None,
                                        sent_time: Utc::now(),
                                        autoconnect,
                                    },
                                );

                                // If QUIT was requested, then ERROR is
                                // a valid acknowledgement
                                // https://modern.ircdocs.horse/#quit-message
                                state = State::Disconnected {
                                    autoconnect,
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
                                        autoconnect,
                                    },
                                );
                                state = State::Disconnected {
                                    autoconnect,
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

                        let autoconnect = quit_requested.is_none();
                        connection_attempt = 0;

                        let _ = sender.unbounded_send(Update::Disconnected {
                            server: server.clone(),
                            is_initial,
                            error: Some(e.to_string()),
                            sent_time: Utc::now(),
                            autoconnect,
                        });
                        state = State::Disconnected {
                            autoconnect,
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

                        let autoconnect = quit_requested.is_none();
                        connection_attempt = 0;

                        let _ = sender.unbounded_send(Update::Disconnected {
                            server: server.clone(),
                            is_initial,
                            error: Some("ping timeout".into()),
                            sent_time: Utc::now(),
                            autoconnect,
                        });
                        state = State::Disconnected {
                            autoconnect,
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
                        Control::Connect(_) | Control::DisableAutoconnect => (),
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
                                retry: time::interval_at(
                                    Instant::now() + config.reconnect_delay,
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
    receiver: tokio::sync::mpsc::UnboundedReceiver<CodecLog>,
) {
    match config.file_format {
        config::server::FileFormat::Plain => {
            match PlainLogWriter::new(&server).await {
                Ok(writer) => log_codec_loop(writer, config, receiver).await,
                Err(err) => log::error!(
                    "unable to create log writer for {server}: {err}"
                ),
            }
        }
        config::server::FileFormat::Gzip => {
            match GzipLogWriter::new(&server).await {
                Ok(writer) => log_codec_loop(writer, config, receiver).await,
                Err(err) => log::error!(
                    "unable to create log writer for {server}: {err}"
                ),
            }
        }
    }
}

async fn log_codec_loop<E: LogEncoder>(
    mut log_writer: LogWriter<E>,
    config: config::server::IrcProtocolLog,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<CodecLog>,
) {
    let mut set_timeout_to_flush = false;

    while let Some(action) = if set_timeout_to_flush {
        tokio::time::timeout(Duration::from_millis(500), receiver.recv())
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

pub trait LogEncoder {
    type Writer: AsyncWrite + Unpin;

    fn extension() -> &'static str;
    fn wrap(file: File) -> Self::Writer;
}

pub struct PlainEncoder;

impl LogEncoder for PlainEncoder {
    type Writer = BufWriter<File>;

    fn extension() -> &'static str {
        "log"
    }

    fn wrap(file: File) -> Self::Writer {
        BufWriter::new(file)
    }
}

pub struct GzipFormatEncoder;

impl LogEncoder for GzipFormatEncoder {
    type Writer = GzipEncoder<File>;

    fn extension() -> &'static str {
        "log.gz"
    }

    fn wrap(file: File) -> Self::Writer {
        GzipEncoder::new(file)
    }
}

pub type PlainLogWriter = LogWriter<PlainEncoder>;
pub type GzipLogWriter = LogWriter<GzipFormatEncoder>;

pub struct LogWriter<E: LogEncoder> {
    log_dir: PathBuf,
    date_writer: Option<(NaiveDate, E::Writer)>,
}

impl<E: LogEncoder> LogWriter<E> {
    async fn new(server: &Server) -> Result<Self, std::io::Error> {
        let data_dir = environment::data_dir();

        let log_dir =
            data_dir.join("irc_protocol_logs").join(server.to_string());

        fs::create_dir_all(&log_dir).await?;

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

        if let Some((date, _)) = &mut self.date_writer {
            if today != *date {
                self.shutdown().await;

                self.create_date_writer(today).await;
            }
        } else {
            self.create_date_writer(today).await;
        }

        if let Some((_, writer)) = &mut self.date_writer {
            let _ = writer
                .write_all(Self::format(message, now, config).as_bytes())
                .await;
        }
    }

    async fn create_date_writer(&mut self, date: NaiveDate) {
        let log_file =
            format!("{}.{}", date.format("%Y-%m-%d"), E::extension());

        self.date_writer = File::options()
            .append(true)
            .create(true)
            .open(self.log_dir.join(log_file))
            .await
            .ok()
            .map(|writer| (date, E::wrap(writer)));
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
        if let Some((_, mut writer)) = self.date_writer.take() {
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

    pub fn authentication_failed(
        &mut self,
        server: &Server,
        error: Option<String>,
    ) {
        if let Some(controller) = self.0.get_mut(server) {
            let _ =
                controller.try_send(Control::AuthenticationFailed { error });
        }
    }

    pub fn connect(&mut self, server: &Server) {
        if let Some(controller) = self.0.get_mut(server) {
            let _ = controller.try_send(Control::Connect(false));
        }
    }

    pub fn disable_autoconnect(&mut self, server: &Server) {
        if let Some(controller) = self.0.get_mut(server) {
            let _ = controller.try_send(Control::DisableAutoconnect);
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
