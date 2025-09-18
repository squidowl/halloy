use std::collections::{HashMap, VecDeque};
use std::num::NonZeroU16;
use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use futures::StreamExt;
use futures::stream::BoxStream;
use itertools::Itertools;
use rand::Rng;

use super::{
    Direction, FileTransfer, Id, ReceiveRequest, SendRequest, Status, Task,
    task,
};
use crate::{Config, dcc};

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

#[derive(Default)]
pub struct Manager {
    items: HashMap<Id, Item>,
    /// Queued = waiting for port assignment
    queued: VecDeque<Id>,
    used_ports: HashMap<Id, NonZeroU16>,
}

impl Manager {
    fn get_random_id(&self) -> Id {
        let mut rng = rand::rng();

        loop {
            let id = Id(rng.random());

            if !self.items.contains_key(&id) {
                return id;
            }
        }
    }

    fn server(&self, config: &Config) -> Option<task::Server> {
        config
            .file_transfer
            .server
            .as_ref()
            .map(|server| task::Server {
                public_address: server.public_address,
                bind_address: server.bind_address,
            })
    }

    pub fn send(
        &mut self,
        request: SendRequest,
        config: &Config,
    ) -> Option<Event> {
        let SendRequest {
            to,
            path,
            server,
            server_handle,
        } = request;

        let reverse = config.file_transfer.passive;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .replace(' ', "_");

        log::debug!(
            "File transfer send request to {} for {filename:?}",
            to.nickname()
        );

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
        let (handle, stream) = task.spawn(
            self.server(config),
            Duration::from_secs(config.file_transfer.timeout),
            config.proxy.clone(),
        );

        self.items.insert(
            id,
            Item::Working {
                file_transfer: file_transfer.clone(),
                task: handle,
            },
        );

        Some(Event::NewTransfer(file_transfer, stream.boxed()))
    }

    pub fn receive(
        &mut self,
        request: ReceiveRequest,
        config: &Config,
    ) -> Option<Event> {
        let ReceiveRequest {
            from,
            dcc_send,
            server,
            server_handle,
        } = request;

        // Check if this is the response to a reverse send we sent
        if let Some(id) = dcc_send.token().and_then(|s| s.parse().ok().map(Id))
            && let dcc::Send::Reverse {
                filename,
                host,
                port: Some(port),
                ..
            } = &dcc_send
            && let Some(Item::Working {
                file_transfer,
                task,
            }) = self.items.get_mut(&id)
            && file_transfer.filename == *filename
        {
            log::debug!(
                "File transfer received reverse confirmation from {} for {:?}",
                from.nickname(),
                filename,
            );
            task.confirm_reverse(*host, *port);
            return None;
        }

        log::debug!(
            "File transfer request received from {} for {:?}",
            from.nickname(),
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

        let task = Task::receive(id, dcc_send, from.clone(), server_handle);
        let (mut handle, stream) = task.spawn(
            self.server(config),
            Duration::from_secs(config.file_transfer.timeout),
            config.proxy.as_ref().cloned(),
        );

        // Auto-accept if enabled and save directory is set
        if config.file_transfer.auto_accept.enabled {
            // Check if sender matches nickname or mask filters
            let should_auto_accept = {
                let nickname_match =
                    config.file_transfer.auto_accept.nicks.as_ref().is_none_or(
                        |nicks| nicks.contains(&from.nickname().to_string()),
                    );

                let mask_match = config
                    .file_transfer
                    .auto_accept
                    .masks
                    .as_ref()
                    .is_none_or(|masks| from.matches_masks(masks));

                nickname_match && mask_match
            };

            if should_auto_accept {
                if let Some(save_directory) =
                    &config.file_transfer.save_directory
                {
                    let save_path =
                        save_directory.join(&file_transfer.filename);

                    log::debug!(
                        "Auto-accepting file transfer from {} for {:?}",
                        from.nickname(),
                        file_transfer.filename
                    );

                    handle.approve(save_path);
                } else {
                    log::warn!(
                        "Auto-accept is enabled but save_directory is not set. File transfer will require manual approval."
                    );
                }
            }
        }

        self.items.insert(
            id,
            Item::Working {
                file_transfer: file_transfer.clone(),
                task: handle,
            },
        );

        Some(Event::NewTransfer(file_transfer, stream.boxed()))
    }

    pub fn update(&mut self, update: task::Update, config: &Config) {
        match update {
            task::Update::Metadata(id, size) => {
                if let Some(item) = self.items.get_mut(&id) {
                    item.file_transfer_mut().size = size;
                }
            }
            task::Update::Queued(id) => {
                let available_port = self.get_available_port(config);

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
                        file_transfer.remote_user.nickname(),
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
                if let Some(Item::Working { file_transfer, .. }) =
                    self.items.remove(&id)
                {
                    log::debug!(
                        "File transfer completed {} {} for {:?} in {:.2}s",
                        match file_transfer.direction {
                            Direction::Sent => "to",
                            Direction::Received => "from",
                        },
                        &file_transfer.remote_user.nickname(),
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
                        &file_transfer.remote_user.nickname(),
                        &file_transfer.filename,
                    );
                    file_transfer.status = Status::Failed { error };

                    self.recycle_port(id);
                }
            }
        }
    }

    fn get_available_port(&self, config: &Config) -> Option<NonZeroU16> {
        let server = config.file_transfer.server.as_ref()?;

        server
            .bind_ports
            .clone()
            .find(|port| {
                !self.used_ports.values().any(|used| used.get() == *port)
            })
            .and_then(NonZeroU16::new)
    }

    fn recycle_port(&mut self, id: Id) {
        if let Some(port) = self.used_ports.remove(&id)
            && let Some(Item::Working {
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
