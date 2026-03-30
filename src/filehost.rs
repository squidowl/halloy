use std::path::PathBuf;
use std::sync::Arc;

use data::target::Target;
use data::{client, fileupload};
use iced::Task;
use iced::widget::pane_grid;

use crate::window;

#[derive(Debug)]
pub enum Message {
    UrlReady {
        window: window::Id,
        pane_id: pane_grid::Pane,
        target: Target,
        url: String,
    },
    UploadFailed {
        window: window::Id,
        pane_id: pane_grid::Pane,
        server: data::Server,
        target: Target,
        error: String,
    },
    KnownSaved(Result<(), data::known_filehosts::Error>),
}

pub struct PendingUpload {
    pub window: window::Id,
    pub pane_id: pane_grid::Pane,
    pub server: data::Server,
    pub target: Target,
    pub upload_url: String,
    pub has_credentials: bool,
    pub file_paths: Vec<PathBuf>,
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
    pending: Option<PendingUpload>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            file_being_hovered: false,
            known: data::KnownFilehosts::load(),
            pending: None,
        }
    }

    /// Called when buffer wants to upload files.
    /// Prompts the user for confirmation if untrusted filehost
    pub fn upload(
        &mut self,
        pending: PendingUpload,
        clients: &client::Map,
        http_client: Arc<reqwest::Client>,
    ) -> (Task<Message>, Option<Event>) {
        let upload_url = pending.upload_url.clone();
        let has_credentials = pending.has_credentials;
        let window = pending.window;

        if self.known.contains(&upload_url) {
            let irc_uses_tls = clients.get_use_tls(&pending.server);
            (
                start_tasks(pending, clients, irc_uses_tls, http_client),
                None,
            )
        } else {
            self.pending = Some(pending);
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
        http_client: Arc<reqwest::Client>,
    ) -> Task<Message> {
        let Some(pending) = self.pending.take() else {
            return Task::none();
        };

        self.known.insert(pending.upload_url.clone());

        let irc_uses_tls = clients.get_use_tls(&pending.server);
        let upload_task =
            start_tasks(pending, clients, irc_uses_tls, http_client);

        let known = self.known.clone();
        let save_task = Task::perform(
            async move { known.save().await },
            Message::KnownSaved,
        );

        Task::batch(vec![upload_task, save_task])
    }

    /// Handle user cancel
    pub fn cancel(&mut self) -> Task<Message> {
        let Some(pending) = self.pending.take() else {
            return Task::none();
        };

        let tasks: Vec<_> = (0..pending.file_paths.len())
            .map(|_| {
                Task::done(Message::UrlReady {
                    window: pending.window,
                    pane_id: pending.pane_id,
                    target: pending.target.clone(),
                    url: String::new(),
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
) -> Task<Message> {
    let PendingUpload {
        window,
        pane_id,
        server,
        target,
        upload_url,
        has_credentials: _,
        file_paths,
        abort_registrations,
    } = pending;

    let tasks: Vec<_> = file_paths
        .into_iter()
        .zip(abort_registrations)
        .map(|(file_path, registration)| {
            let upload_url = upload_url.clone();
            let auth = clients.get_filehost_auth(&server);
            let http_client = http_client.clone();
            let server = server.clone();
            let target = target.clone();

            Task::perform(
                async move {
                    let fut = fileupload::upload(
                        &upload_url,
                        &file_path,
                        auth,
                        irc_uses_tls,
                        http_client,
                    );
                    futures::future::Abortable::new(fut, registration).await
                },
                move |result| match result {
                    Ok(Ok(url)) => Message::UrlReady {
                        window,
                        pane_id,
                        target,
                        url,
                    },
                    Ok(Err(e)) => {
                        log::warn!("filehost upload failed: {e}");
                        Message::UploadFailed {
                            window,
                            pane_id,
                            server,
                            target,
                            error: e.to_string(),
                        }
                    }
                    Err(_aborted) => Message::UrlReady {
                        window,
                        pane_id,
                        target,
                        url: String::new(),
                    },
                },
            )
        })
        .collect();

    Task::batch(tasks)
}
