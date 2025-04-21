use std::{path::PathBuf, time::Instant};

use data::{Server, config};
use iced::Task;

use crate::widget::Element;

pub mod connect_to_server;
pub mod image_preview;
pub mod prompt_before_open_url;
pub mod reload_configuration_error;

#[derive(Debug)]
pub enum Modal {
    ReloadConfigurationError(config::Error),
    ServerConnect {
        url: String,
        server: Server,
        config: config::Server,
    },
    PromptBeforeOpenUrl(String),
    ImagePreview {
        source: PathBuf,
        url: url::Url,
        timer: Option<Instant>,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Cancel,
    OpenURL(String),
    // Modal specific messages
    ServerConnect(ServerConnect),
    ImagePreview(ImagePreview),
}

#[derive(Debug, Clone)]
pub enum ImagePreview {
    SaveImage(PathBuf),
    SavedImage(Option<PathBuf>),
}

#[derive(Debug, Clone)]
pub enum ServerConnect {
    AcceptNewServer,
    DangerouslyAcceptInvalidCerts(bool),
}

pub enum Event {
    CloseModal,
    AcceptNewServer,
}

impl Modal {
    pub fn update(
        &mut self,
        message: Message,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Cancel => (Task::none(), Some(Event::CloseModal)),
            Message::ServerConnect(server_connect) => match server_connect {
                ServerConnect::AcceptNewServer => {
                    (Task::none(), Some(Event::AcceptNewServer))
                }
                ServerConnect::DangerouslyAcceptInvalidCerts(toggle) => {
                    if let Modal::ServerConnect { config, .. } = self {
                        config.dangerously_accept_invalid_certs = toggle;
                    }

                    (Task::none(), None)
                }
            },
            Message::OpenURL(url) => {
                let _ = open::that_detached(url);
                (Task::none(), Some(Event::CloseModal))
            }
            Message::ImagePreview(image_preview) => match image_preview {
                ImagePreview::SaveImage(source) => (
                    Task::perform(
                        async move {
                            if let Some(handle) = rfd::AsyncFileDialog::new()
                                .set_file_name(
                                    source
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or_default(),
                                )
                                .save_file()
                                .await
                            {
                                let destination = handle.path();
                                tokio::fs::copy(&source, destination)
                                    .await
                                    .ok()
                                    .map(|_| destination.to_path_buf())
                            } else {
                                None
                            }
                        },
                        move |path| {
                            Message::ImagePreview(ImagePreview::SavedImage(
                                path,
                            ))
                        },
                    ),
                    None,
                ),
                ImagePreview::SavedImage(path) => {
                    if path.is_some() {
                        if let Modal::ImagePreview { timer, .. } = self {
                            *timer = Some(Instant::now());
                        }
                    }

                    (Task::none(), None)
                }
            },
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self {
            Modal::ReloadConfigurationError(error) => {
                reload_configuration_error::view(error)
            }
            Modal::ServerConnect {
                url: raw, config, ..
            } => connect_to_server::view(raw, config),
            Modal::PromptBeforeOpenUrl(payload) => {
                prompt_before_open_url::view(payload)
            }
            Modal::ImagePreview { source, url, timer } => {
                image_preview::view(source, url, timer)
            }
        }
    }
}
