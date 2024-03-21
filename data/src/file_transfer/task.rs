use std::{
    io,
    net::IpAddr,
    num::NonZeroU16,
    path::PathBuf,
    time::{Duration, Instant},
};

use bytes::Bytes;
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    SinkExt, Stream,
};
use irc::{connection, BytesCodec, Connection};
use thiserror::Error;
use tokio::{fs::File, io::AsyncWriteExt, task::JoinHandle};
use tokio_stream::StreamExt;

use super::Id;
use crate::{dcc, server};

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
    },
}

impl Task {
    pub fn receive(id: Id, dcc_send: dcc::Send, server_handle: server::Handle) -> Self {
        Self::Receive {
            id,
            dcc_send,
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
                    server_handle,
                } => {
                    if let Err(error) =
                        receive(id, dcc_send, server_handle, action_receiver, update_sender).await
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
}

#[derive(Debug)]
pub enum Update {
    Metadata(Id, u64),
    Failed(Id, String),
    Finished {
        id: Id,
        elapsed: Duration,
        sha256: String,
    },
}

async fn receive(
    id: Id,
    dcc_send: dcc::Send,
    _server_handle: server::Handle,
    mut action: Receiver<Action>,
    mut update: Sender<Update>,
) -> Result<(), Error> {
    // Wait for approval
    let Some(Action::Approve { save_to }) = action.next().await else {
        return Ok(());
    };

    let dcc::Send::Direct { host, port, .. } = dcc_send else {
        return Ok(());
    };

    let host = host.to_string();

    let started_at = Instant::now();

    let mut connection = Connection::new(
        connection::Config {
            server: &host,
            port: port.get(),
            // TODO: TLS?
            security: connection::Security::Unsecured,
        },
        BytesCodec::new(),
    )
    .await?;

    let mut file = File::create(&save_to).await?;

    let mut bytes_received = 0;

    while let Some(bytes) = connection.next().await {
        let bytes = bytes?;

        bytes_received += bytes.len();

        file.write_all(&bytes).await?;

        let ack = Bytes::from_iter(((bytes_received as u64 & 0xFFFFFFFF) as u32).to_be_bytes());
        connection.send(ack).await?;
    }

    let elapsed = started_at.elapsed();

    let _ = update
        .send(Update::Finished {
            id,
            elapsed,
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
