use std::path::PathBuf;
use std::sync::Arc;

use data::config::{Proxy, proxy};
use data::target::Target;
use data::{client, fileupload};
use iced::Task;
use iced::widget::pane_grid;

use crate::window;

#[derive(Debug)]
pub enum Message {
    UploadDone {
        window: window::Id,
        pane_id: pane_grid::Pane,
        target: Option<Target>,
        id: u32,
        url: Option<String>,
    },
    UploadFailed {
        window: window::Id,
        pane_id: pane_grid::Pane,
        server: data::Server,
        target: Option<Target>,
        id: u32,
        error: String,
    },
    KnownSaved(Result<(), data::known_filehosts::Error>),
}

pub struct PendingUpload {
    pub window: window::Id,
    pub pane_id: pane_grid::Pane,
    pub server: data::Server,
    pub target: Option<Target>,
    pub upload_url: String,
    pub is_override_url: bool,
    pub has_credentials: bool,
    pub file_paths: Vec<PathBuf>,
    pub upload_ids: Vec<u32>,
    pub abort_registrations: Vec<futures::future::AbortRegistration>,
}

#[derive(Debug)]
pub enum Event {
    PromptBeforeUpload {
        upload_url: String,
        has_credentials: bool,
        window: window::Id,
    },
}

pub struct Manager {
    pub file_being_hovered: bool,
    known: data::KnownFilehosts,
    pending: Vec<PendingUpload>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            file_being_hovered: false,
            known: data::KnownFilehosts::load(),
            pending: vec![],
        }
    }

    /// Called when buffer wants to upload files.
    /// Prompts the user for confirmation if untrusted filehost
    pub fn upload(
        &mut self,
        pending: PendingUpload,
        clients: &client::Map,
        default_http_client: Option<Arc<reqwest::Client>>,
        default_proxy_config: &Option<Proxy>,
    ) -> (Task<Message>, Option<Event>) {
        let upload_url = pending.upload_url.clone();
        let has_credentials = pending.has_credentials;
        let window = pending.window;

        if self.known.contains(&upload_url) {
            let irc_uses_tls = clients.get_use_tls(&pending.server);
            let (http_client, proxy_config) = if let Some(proxy_config) =
                clients.get_server_proxy_config(&pending.server)
            {
                (
                    clients.get_server_http_client(&pending.server),
                    Some(proxy_config),
                )
            } else {
                (default_http_client, default_proxy_config.as_ref())
            };

            let Some(http_client) = http_client else {
                // Detailed HTTP client build error should already be in the logs
                log::warn!(
                    "[{}] File upload disabled: Unable to build HTTP client",
                    pending.server
                );
                return (Task::none(), None);
            };

            (
                start_tasks(
                    pending,
                    clients,
                    irc_uses_tls,
                    http_client,
                    proxy_config,
                ),
                None,
            )
        } else {
            self.pending.push(pending);
            (
                Task::none(),
                Some(Event::PromptBeforeUpload {
                    upload_url,
                    has_credentials,
                    window,
                }),
            )
        }
    }

    /// Handle user confirm
    pub fn proceed(
        &mut self,
        clients: &client::Map,
        default_http_client: Option<Arc<reqwest::Client>>,
        default_proxy_config: &Option<proxy::Proxy>,
    ) -> Task<Message> {
        let pending = std::mem::take(&mut self.pending);
        if pending.is_empty() {
            return Task::none();
        }

        let tasks: Vec<_> = pending
            .into_iter()
            .map(|p| {
                self.known.insert(p.upload_url.clone());

                let irc_uses_tls = clients.get_use_tls(&p.server);
                let (http_client, proxy_config) = if let Some(proxy_config) =
                    clients.get_server_proxy_config(&p.server)
                {
                    (
                        clients.get_server_http_client(&p.server),
                        Some(proxy_config),
                    )
                } else {
                    (default_http_client.clone(), default_proxy_config.as_ref())
                };

                let Some(http_client) = http_client else {
                    // Detailed HTTP client build error should already be in the logs
                    log::warn!("[{}] File upload disabled: Unable to build HTTP client", p.server);
                    return Task::none();
                };

                start_tasks(p, clients, irc_uses_tls, http_client, proxy_config)
            })
            .collect();

        let known = self.known.clone();
        let save_task = Task::perform(
            async move { known.save().await },
            Message::KnownSaved,
        );

        Task::batch(tasks.into_iter().chain([save_task]).collect::<Vec<_>>())
    }

    /// Handle user cancel
    pub fn cancel(&mut self) -> Task<Message> {
        let pending = std::mem::take(&mut self.pending);
        if pending.is_empty() {
            return Task::none();
        }

        let tasks: Vec<_> = pending
            .into_iter()
            .flat_map(|p| {
                p.upload_ids.into_iter().map(move |id| {
                    Task::done(Message::UploadDone {
                        window: p.window,
                        pane_id: p.pane_id,
                        target: p.target.clone(),
                        id,
                        url: None,
                    })
                })
            })
            .collect();

        Task::batch(tasks)
    }
}

fn start_tasks(
    pending: PendingUpload,
    clients: &client::Map,
    irc_uses_tls: bool,
    http_client: Arc<reqwest::Client>,
    proxy_config: Option<&proxy::Proxy>,
) -> Task<Message> {
    let PendingUpload {
        window,
        pane_id,
        server,
        target,
        upload_url,
        is_override_url,
        has_credentials: _,
        file_paths,
        upload_ids,
        abort_registrations,
    } = pending;

    let tasks: Vec<_> = file_paths
        .into_iter()
        .zip(upload_ids)
        .zip(abort_registrations)
        .map(|((file_path, id), registration)| {
            let upload_url = upload_url.clone();
            let auth = clients.get_filehost_auth(&server);
            let http_client = http_client.clone();
            let server = server.clone();
            let target = target.clone();
            let proxy_config = proxy_config.cloned();

            Task::perform(
                async move {
                    let fut = fileupload::upload(
                        &upload_url,
                        &file_path,
                        auth,
                        irc_uses_tls,
                        is_override_url,
                        http_client,
                        proxy_config.as_ref(),
                    );
                    futures::future::Abortable::new(fut, registration).await
                },
                move |result| match result {
                    Ok(Ok(url)) => Message::UploadDone {
                        window,
                        pane_id,
                        target,
                        id,
                        url: Some(url),
                    },
                    Ok(Err(e)) => {
                        log::warn!("filehost upload failed: {e}");
                        Message::UploadFailed {
                            window,
                            pane_id,
                            server,
                            target,
                            id,
                            error: e.to_string(),
                        }
                    }
                    Err(_aborted) => Message::UploadDone {
                        window,
                        pane_id,
                        target,
                        id,
                        url: None,
                    },
                },
            )
        })
        .collect();

    Task::batch(tasks)
}
