use std::{
    collections::{HashMap, VecDeque},
    num::NonZeroU16,
    path::PathBuf,
    time::Duration,
};

use chrono::Utc;
use futures::{stream::BoxStream, StreamExt};
use itertools::Itertools;
use rand::Rng;

use super::{task, Direction, FileTransfer, Id, ReceiveRequest, SendRequest, Status, Task};
use crate::{config, dcc};

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
    NewTransfer(FileTransfer, BoxStream<'static, task::Update>),
}

pub struct Manager {
    config: config::FileTransfer,
    items: HashMap<Id, Item>,
    /// Queued = waiting for port assignment
    queued: VecDeque<Id>,
    used_ports: HashMap<Id, NonZeroU16>,
}

impl Manager {
    pub fn new(config: config::FileTransfer) -> Self {
        Self {
            config,
            items: HashMap::new(),
            queued: VecDeque::new(),
            used_ports: HashMap::new(),
        }
    }

    fn get_random_id(&self) -> Id {
        let mut rng = rand::thread_rng();

        loop {
            let id = Id(rng.gen());

            if !self.items.contains_key(&id) {
                return id;
            }
        }
    }

    fn server(&self) -> Option<task::Server> {
        self.config.server.as_ref().map(|server| task::Server {
            public_address: server.public_address,
            bind_address: server.bind_address,
        })
    }

    pub fn send(&mut self, request: SendRequest) -> Option<Event> {
        let SendRequest {
            to,
            path,
            server,
            server_handle,
        } = request;

        let reverse = self.config.passive;

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
            filename: filename.clone(),
            // Will be updated by task
            size: 0,
            status: if reverse {
                Status::PendingReverseConfirmation
            } else {
                // Task will trigger queued update
                Status::Queued
            },
        };

        let task = Task::send(id, path, filename, to, reverse, server_handle);
        let (handle, stream) = task.spawn(self.server(), Duration::from_secs(self.config.timeout));

        self.items.insert(
            id,
            Item::Working {
                file_transfer: file_transfer.clone(),
                task: handle,
            },
        );

        Some(Event::NewTransfer(file_transfer, stream.boxed()))
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
            if let dcc::Send::Reverse {
                filename,
                host,
                port: Some(port),
                ..
            } = &dcc_send
            {
                if let Some(Item::Working {
                    file_transfer,
                    task,
                }) = self.items.get_mut(&id)
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
            filename: dcc_send.filename().to_string(),
            size: dcc_send.size(),
            status: Status::PendingApproval,
        };

        let task = Task::receive(id, dcc_send, from, server_handle);
        let (handle, stream) = task.spawn(self.server(), Duration::from_secs(self.config.timeout));

        self.items.insert(
            id,
            Item::Working {
                file_transfer: file_transfer.clone(),
                task: handle,
            },
        );

        Some(Event::NewTransfer(file_transfer, stream.boxed()))
    }

    pub fn update(&mut self, update: task::Update) {
        match update {
            task::Update::Metadata(id, size) => {
                if let Some(item) = self.items.get_mut(&id) {
                    item.file_transfer_mut().size = size;
                }
            }
            task::Update::Queued(id) => {
                let available_port = self.get_available_port();

                if let Some(Item::Working {
                    file_transfer,
                    task,
                }) = self.items.get_mut(&id)
                {
                    if let Some(port) = available_port {
                        task.port_available(port);
                        self.used_ports.insert(id, port);
                    } else {
                        // If port is not available, queue the item so it
                        // can be assigned the next available port
                        file_transfer.status = Status::Queued;
                        self.queued.push_back(id);
                    }
                }
            }
            task::Update::Ready(id) => {
                if let Some(item) = self.items.get_mut(&id) {
                    item.file_transfer_mut().status = Status::Ready;
                }
            }
            task::Update::Progress {
                id,
                transferred,
                elapsed,
            } => {
                if let Some(item) = self.items.get_mut(&id) {
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
                if let Some(Item::Working { file_transfer, .. }) = self.items.remove(&id) {
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

                    self.items.insert(
                        id,
                        Item::Finished(FileTransfer {
                            status: Status::Completed { elapsed, sha256 },
                            ..file_transfer
                        }),
                    );

                    self.recycle_port(id);
                }
            }
            task::Update::Failed(id, error) => {
                if let Some(item) = self.items.get_mut(&id) {
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

                    self.recycle_port(id);
                }
            }
        }
    }

    fn get_available_port(&self) -> Option<NonZeroU16> {
        let server = self.config.server.as_ref()?;

        server
            .bind_ports
            .clone()
            .find(|port| !self.used_ports.values().any(|used| used.get() == *port))
            .and_then(NonZeroU16::new)
    }

    fn recycle_port(&mut self, id: Id) {
        if let Some(port) = self.used_ports.remove(&id) {
            if let Some(Item::Working {
                task,
                file_transfer,
            }) = self
                .queued
                .pop_front()
                .and_then(|id| self.items.get_mut(&id))
            {
                task.port_available(port);
                self.used_ports.insert(file_transfer.id, port);
            }
        }
    }

    pub fn remove(&mut self, id: &Id) {
        let _ = self.items.remove(id);
        self.queued.retain(|i| i != id);
        self.recycle_port(*id);
    }

    pub fn approve(&mut self, id: &Id, save_to: PathBuf) {
        if let Some(Item::Working { task, .. }) = self.items.get_mut(id) {
            task.approve(save_to);
        }
    }

    pub fn get<'a>(&'a self, id: &Id) -> Option<&'a FileTransfer> {
        self.items.get(id).map(Item::file_transfer)
    }

    pub fn list(&self) -> impl Iterator<Item = &'_ FileTransfer> {
        self.items.values().map(Item::file_transfer).sorted()
    }

    pub fn is_empty(&self) -> bool {
        self.items.values().len() == 0
    }
}
