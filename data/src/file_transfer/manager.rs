use std::{collections::HashMap, path::PathBuf};

use chrono::Utc;
use futures::{stream::BoxStream, StreamExt};
use itertools::Itertools;
use rand::Rng;

use super::{task, Direction, FileTransfer, Id, ReceiveRequest, SendRequest, Status, Task};
use crate::dcc;

enum Item {
    Working {
        file_transfer: FileTransfer,
        task: task::Handle,
    },
    Finished(FileTransfer),
}

impl Item {
    fn file_transfer(&self) -> &FileTransfer {
        match self {
            Item::Working { file_transfer, .. } => file_transfer,
            Item::Finished(file_transfer) => file_transfer,
        }
    }

    fn file_transfer_mut(&mut self) -> &mut FileTransfer {
        match self {
            Item::Working { file_transfer, .. } => file_transfer,
            Item::Finished(file_transfer) => file_transfer,
        }
    }
}

pub enum Event {
    RunTask(BoxStream<'static, task::Update>),
}

#[derive(Default)]
pub struct Manager(HashMap<Id, Item>);

impl Manager {
    fn get_random_id(&self) -> Id {
        let mut rng = rand::thread_rng();

        loop {
            let id = Id(rng.gen());

            if !self.0.contains_key(&id) {
                return id;
            }
        }
    }

    pub fn send(&mut self, request: SendRequest) -> Option<Event> {
        let SendRequest {
            to,
            path,
            server,
            server_handle,
        } = request;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .replace(' ', "_");

        log::debug!("File transfer send request to {to} for {filename:?}");

        let id = self.get_random_id();

        // Otherwise this must be a new request
        let file_transfer = FileTransfer {
            id,
            server,
            created_at: Utc::now(),
            direction: Direction::Sent,
            remote_user: to.clone(),
            // TODO: Do we want to support this?
            secure: false,
            filename: filename.clone(),
            // Will be updated by task
            size: 0,
            status: Status::Pending,
        };

        let task = Task::send(id, path, filename, to, server_handle);
        let (handle, stream) = task.spawn();

        self.0.insert(
            id,
            Item::Working {
                file_transfer,
                task: handle,
            },
        );

        Some(Event::RunTask(stream.boxed()))
    }

    pub fn receive(&mut self, request: ReceiveRequest) -> Option<Event> {
        let ReceiveRequest {
            from,
            dcc_send,
            server,
            server_handle,
        } = request;

        // Check if this is the response to a reverse send we sent
        if let Some(id) = dcc_send.token().and_then(|s| s.parse().ok().map(Id)) {
            if let dcc::Send::Direct {
                filename,
                host,
                port,
                ..
            } = &dcc_send
            {
                if let Some(Item::Working {
                    file_transfer,
                    task,
                }) = self.0.get_mut(&id)
                {
                    if file_transfer.filename == *filename {
                        log::debug!(
                            "File transfer received reverse confirmation from {from} for {:?}",
                            filename,
                        );
                        task.confirm_reverse(*host, *port);
                        return None;
                    }
                }
            }
        }

        log::debug!(
            "File transfer request received from {from} for {:?}",
            dcc_send.filename()
        );

        let id = self.get_random_id();

        // Otherwise this must be a new request
        let file_transfer = FileTransfer {
            id,
            server,
            created_at: Utc::now(),
            direction: Direction::Received,
            remote_user: from.clone(),
            secure: dcc_send.secure(),
            filename: dcc_send.filename().to_string(),
            size: dcc_send.size(),
            status: Status::Pending,
        };

        let task = Task::receive(id, dcc_send, from, server_handle);
        let (handle, stream) = task.spawn();

        self.0.insert(
            id,
            Item::Working {
                file_transfer,
                task: handle,
            },
        );

        Some(Event::RunTask(stream.boxed()))
    }

    pub fn update(&mut self, update: task::Update) {
        match update {
            task::Update::Metadata(id, size) => {
                if let Some(item) = self.0.get_mut(&id) {
                    item.file_transfer_mut().size = size;
                }
            }
            task::Update::Progress {
                id,
                transferred,
                elapsed,
            } => {
                if let Some(item) = self.0.get_mut(&id) {
                    let file_transfer = item.file_transfer_mut();
                    log::trace!(
                        "File transfer progress {} {} for {:?}: {:>4.1}%",
                        match file_transfer.direction {
                            Direction::Sent => "to",
                            Direction::Received => "from",
                        },
                        file_transfer.remote_user,
                        file_transfer.filename,
                        transferred as f32 / file_transfer.size as f32 * 100.0,
                    );
                    file_transfer.status = Status::Active {
                        transferred,
                        elapsed,
                    };
                }
            }
            task::Update::Finished {
                id,
                elapsed,
                sha256,
            } => {
                if let Some(Item::Working { file_transfer, .. }) = self.0.remove(&id) {
                    log::debug!(
                        "File transfer completed {} {} for {:?} in {:.2}s",
                        match file_transfer.direction {
                            Direction::Sent => "to",
                            Direction::Received => "from",
                        },
                        &file_transfer.remote_user,
                        &file_transfer.filename,
                        elapsed.as_secs_f32()
                    );

                    self.0.insert(
                        id,
                        Item::Finished(FileTransfer {
                            status: Status::Completed { elapsed, sha256 },
                            ..file_transfer
                        }),
                    );
                }
            }
            task::Update::Failed(id, error) => {
                if let Some(item) = self.0.get_mut(&id) {
                    let file_transfer = item.file_transfer_mut();
                    log::error!(
                        "File transfer failed {} {} for {:?}: {error}",
                        match file_transfer.direction {
                            Direction::Sent => "to",
                            Direction::Received => "from",
                        },
                        &file_transfer.remote_user,
                        &file_transfer.filename,
                    );
                    file_transfer.status = Status::Failed { error };
                }
            }
        }
    }

    pub fn approve(&mut self, id: &Id, save_to: PathBuf) {
        if let Some(Item::Working { task, .. }) = self.0.get_mut(id) {
            task.approve(save_to);
        }
    }

    pub fn get<'a>(&'a self, id: &Id) -> Option<&'a FileTransfer> {
        self.0.get(id).map(Item::file_transfer)
    }

    pub fn list(&self) -> impl Iterator<Item = &'_ FileTransfer> {
        self.0.values().map(Item::file_transfer).sorted()
    }
}