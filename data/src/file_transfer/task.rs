use std::{
    io,
    net::IpAddr,
    num::NonZeroU16,
    path::PathBuf,
    time::{Duration, Instant},
};

use bytes::{Bytes, BytesMut};
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    SinkExt, Stream,
};
use irc::{connection, BytesCodec, Connection};
use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    task::JoinHandle,
};
use tokio_stream::StreamExt;

use super::Id;
use crate::{dcc, server, user::Nick};

/// 16 KiB
pub const BUFFER_SIZE: usize = 16 * 1024;

pub struct Handle {
    sender: Sender<Action>,
    task: JoinHandle<()>,
}

impl Handle {
    pub fn approve(&mut self, save_to: PathBuf) {
        let _ = self.sender.try_send(Action::Approve { save_to });
    }

    pub fn confirm_reverse(&mut self, host: IpAddr, port: NonZeroU16) {
        let _ = self
            .sender
            .try_send(Action::ReverseConfirmed { host, port });
    }

    pub fn port_available(&mut self, port: NonZeroU16) {
        let _ = self.sender.try_send(Action::PortAvailable { port });
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub enum Task {
    Receive {
        id: Id,
        dcc_send: dcc::Send,
        server_handle: server::Handle,
        remote_user: Nick,
    },
    Send {
        id: Id,
        path: PathBuf,
        sanitized_filename: String,
        remote_user: Nick,
        reverse: bool,
        server_handle: server::Handle,
    },
}

impl Task {
    pub fn receive(
        id: Id,
        dcc_send: dcc::Send,
        remote_user: Nick,
        server_handle: server::Handle,
    ) -> Self {
        Self::Receive {
            id,
            dcc_send,
            remote_user,
            server_handle,
        }
    }

    pub fn send(
        id: Id,
        path: PathBuf,
        sanitized_filename: String,
        remote_user: Nick,
        reverse: bool,
        server_handle: server::Handle,
    ) -> Self {
        Self::Send {
            id,
            path,
            sanitized_filename,
            remote_user,
            reverse,
            server_handle,
        }
    }

    pub fn spawn(self) -> (Handle, impl Stream<Item = Update>) {
        let (action_sender, action_receiver) = mpsc::channel(1);
        let (update_sender, update_receiver) = mpsc::channel(100);

        let task = tokio::spawn(async move {
            let mut update = update_sender.clone();

            match self {
                Task::Receive {
                    id,
                    dcc_send,
                    remote_user,
                    server_handle,
                } => {
                    if let Err(error) = receive(
                        id,
                        dcc_send,
                        remote_user,
                        server_handle,
                        action_receiver,
                        update_sender,
                    )
                    .await
                    {
                        let _ = update.send(Update::Failed(id, error.to_string())).await;
                    }
                }
                Task::Send {
                    id,
                    path,
                    sanitized_filename,
                    remote_user,
                    reverse,
                    server_handle,
                } => {
                    if let Err(error) = send(
                        id,
                        path,
                        sanitized_filename,
                        remote_user,
                        // TODO: Config
                        reverse,
                        server_handle,
                        action_receiver,
                        update_sender,
                    )
                    .await
                    {
                        let _ = update.send(Update::Failed(id, error.to_string())).await;
                    }
                }
            }
        });

        (
            Handle {
                sender: action_sender,
                task,
            },
            update_receiver,
        )
    }
}

pub enum Action {
    Approve { save_to: PathBuf },
    ReverseConfirmed { host: IpAddr, port: NonZeroU16 },
    PortAvailable { port: NonZeroU16 },
}

#[derive(Debug)]
pub enum Update {
    Metadata(Id, u64),
    Queued(Id),
    Ready(Id),
    Progress {
        id: Id,
        transferred: u64,
        elapsed: Duration,
    },
    Finished {
        id: Id,
        elapsed: Duration,
        sha256: String,
    },
    Failed(Id, String),
}

async fn receive(
    id: Id,
    dcc_send: dcc::Send,
    remote_user: Nick,
    mut server_handle: server::Handle,
    mut action: Receiver<Action>,
    mut update: Sender<Update>,
) -> Result<(), Error> {
    // Wait for approval
    let Some(Action::Approve { save_to }) = action.next().await else {
        return Ok(());
    };

    let (host, port, reverse) = match dcc_send {
        dcc::Send::Direct { host, port, .. } => (host, port, false),
        dcc::Send::Reverse {
            secure,
            filename,
            size,
            token,
            ..
        } => {
            // TODO: We need to configure these
            let host = IpAddr::V4([127, 0, 0, 1].into());

            let _ = update.send(Update::Queued(id)).await;

            let Some(Action::PortAvailable { port }) = action.next().await else {
                unreachable!();
            };

            let _ = server_handle
                .send(
                    dcc::Send::Reverse {
                        secure,
                        filename,
                        host,
                        port: Some(port),
                        size,
                        token,
                    }
                    .encode(remote_user),
                )
                .await;

            (host, port, true)
        }
    };

    let started_at = Instant::now();

    let _ = update.send(Update::Ready(id)).await;

    let mut connection = if reverse {
        Connection::listen_and_accept(
            host,
            port.get(),
            // TODO: SSL
            connection::Security::Unsecured,
            BytesCodec::new(),
        )
        .await?
    } else {
        Connection::new(
            connection::Config {
                server: &host.to_string(),
                port: port.get(),
                // TODO: TLS?
                security: connection::Security::Unsecured,
            },
            BytesCodec::new(),
        )
        .await?
    };

    let mut file = File::create(&save_to).await?;

    let mut transferred = 0;
    let mut last_progress = started_at;

    while let Some(bytes) = connection.next().await {
        let bytes = bytes?;

        transferred += bytes.len();

        // Write bytes to file
        file.write_all(&bytes).await?;

        // Reply w/ ack
        let ack = Bytes::from_iter(((transferred as u64 & 0xFFFFFFFF) as u32).to_be_bytes());
        connection.send(ack).await?;

        // Send progress at 60fps
        if last_progress.elapsed() >= Duration::from_millis(16) {
            let _ = update
                .send(Update::Progress {
                    id,
                    elapsed: started_at.elapsed(),
                    transferred: transferred as u64,
                })
                .await;
            last_progress = Instant::now();
        }
    }

    let _ = update
        .send(Update::Finished {
            id,
            elapsed: started_at.elapsed(),
            // TODO
            sha256: String::default(),
        })
        .await;

    Ok(())
}

async fn send(
    id: Id,
    path: PathBuf,
    sanitized_filename: String,
    remote_user: Nick,
    reverse: bool,
    mut server_handle: server::Handle,
    mut action: Receiver<Action>,
    mut update: Sender<Update>,
) -> Result<(), Error> {
    let mut file = File::open(path).await?;
    let size = file.metadata().await?.len();

    let _ = update.send(Update::Metadata(id, size)).await;

    let mut connection = if reverse {
        // Host doesn't matter for reverse connection
        let host = IpAddr::V4([127, 0, 0, 1].into());
        let token = u16::from(id).to_string();

        let _ = server_handle
            .send(
                dcc::Send::Reverse {
                    secure: false,
                    filename: sanitized_filename,
                    host,
                    port: None,
                    size,
                    token,
                }
                .encode(remote_user),
            )
            .await;

        let Some(Action::ReverseConfirmed { host, port }) = action.next().await else {
            unreachable!();
        };

        let _ = update.send(Update::Ready(id)).await;

        Connection::new(
            connection::Config {
                server: &host.to_string(),
                port: port.get(),
                security: connection::Security::Unsecured,
            },
            BytesCodec::new(),
        )
        .await?
    } else {
        // TODO: We need to configure these
        let host = IpAddr::V4([127, 0, 0, 1].into());

        let _ = update.send(Update::Queued(id)).await;

        let Some(Action::PortAvailable { port }) = action.next().await else {
            unreachable!();
        };

        let _ = server_handle
            .send(
                dcc::Send::Direct {
                    secure: false,
                    filename: sanitized_filename,
                    host,
                    port,
                    size,
                }
                .encode(remote_user),
            )
            .await;

        let _ = update.send(Update::Ready(id)).await;

        Connection::listen_and_accept(
            host,
            port.get(),
            // TODO: SSL
            connection::Security::Unsecured,
            BytesCodec::new(),
        )
        .await?
    };

    let started_at = Instant::now();

    let mut buffer = BytesMut::with_capacity(BUFFER_SIZE);

    let mut transferred = 0;
    let mut last_progress = started_at;

    while transferred < size {
        let n = file.read_buf(&mut buffer).await?;

        // Write bytes to file
        connection.send(buffer.split().freeze()).await?;

        transferred += n as u64;

        buffer.reserve(BUFFER_SIZE);

        // Send progress at 60fps
        if last_progress.elapsed() >= Duration::from_millis(16) {
            let _ = update
                .send(Update::Progress {
                    id,
                    elapsed: started_at.elapsed(),
                    transferred,
                })
                .await;
            last_progress = Instant::now();
        }
    }

    connection.shutdown().await?;

    let _ = update
        .send(Update::Finished {
            id,
            elapsed: started_at.elapsed(),
            // TODO
            sha256: String::default(),
        })
        .await;

    Ok(())
}

#[derive(Debug, Error)]
enum Error {
    #[error("connection error: {0}")]
    Connection(#[from] connection::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}
