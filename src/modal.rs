use std::path::PathBuf;
use std::time::Instant;

use data::{Image, Server, config};
use iced::Task;

use crate::widget::Element;
use crate::{Theme, image_animation, open_url, window};

pub mod about;
pub mod confirm_file_upload;
pub mod connect_to_server;
pub mod image_preview;
pub mod keyring_password;
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
    About(about::About),
    PromptBeforeOpenUrl {
        url: String,
        window: window::Id,
    },
    ImagePreview {
        image: Image,
        timer: Option<Instant>,
        animation: Option<image_animation::Animation>,
        window: window::Id,
    },
    ConfirmFileUpload {
        url: String,
        has_credentials: bool,
        window: window::Id,
    },
    KeyringPassword(keyring_password::KeyringPassword),
}

#[derive(Debug, Clone)]
pub enum Message {
    Cancel,
    OpenURL(String),
    ConfirmFileUpload,
    // Modal specific messages
    ServerConnect(ServerConnect),
    About(about::Action),
    ImagePreview(ImagePreview),
    KeyringPassword(keyring_password::Action),
}

#[derive(Debug, Clone)]
pub enum ImagePreview {
    SaveImage(PathBuf),
    SavedImage(Option<PathBuf>),
    Animation(image_animation::Message),
}

#[derive(Debug, Clone)]
pub enum ServerConnect {
    AcceptNewServer,
    DangerouslyAcceptInvalidCerts(bool),
}

pub enum Event {
    CloseModal,
    AcceptNewServer,
    ConfirmFileUpload,
    KeyringPasswordStored,
}

impl Modal {
    pub fn image_preview(
        image: Image,
        window: window::Id,
        config: &config::preview::Image,
    ) -> (Self, Task<Message>) {
        let mut modal = Self::ImagePreview {
            image,
            timer: None,
            animation: None,
            window,
        };
        let task = modal.start_image_animation(config);
        (modal, task)
    }

    pub fn start_image_animation(
        &mut self,
        config: &config::preview::Image,
    ) -> Task<Message> {
        if let Self::ImagePreview {
            image, animation, ..
        } = self
            && animation.is_none()
            && config.can_animate()
            && matches!(
                image.format,
                data::image::Format::Raster(image::ImageFormat::Gif)
            )
        {
            let (playback, task) =
                image_animation::Animation::new(image.path.clone());
            *animation = Some(playback);

            task.map(|message| {
                Message::ImagePreview(ImagePreview::Animation(message))
            })
        } else {
            Task::none()
        }
    }

    pub fn stop_image_animation(&mut self) {
        if let Self::ImagePreview { animation, .. } = self {
            *animation = None;
        }
    }

    pub fn window_id(&self) -> Option<window::Id> {
        match self {
            Modal::ReloadConfigurationError(..) => None,
            Modal::ServerConnect { .. } => None,
            Modal::About(..) => None,
            Modal::PromptBeforeOpenUrl { url: _, window } => Some(*window),
            Modal::ImagePreview { window, .. } => Some(*window),
            Modal::ConfirmFileUpload { window, .. } => Some(*window),
            Modal::KeyringPassword(_) => None,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Cancel => (Task::none(), Some(Event::CloseModal)),
            Message::ConfirmFileUpload => {
                (Task::none(), Some(Event::ConfirmFileUpload))
            }
            Message::About(action) => {
                if let Modal::About(about) = self {
                    (about.update(action), None)
                } else {
                    (Task::none(), None)
                }
            }
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
            Message::OpenURL(raw_url) => {
                let canonical = url::Url::parse(&raw_url)
                    .map_or(raw_url, |u| u.to_string());
                let _ = open_url::open(canonical);
                let close = !matches!(self, Modal::ConfirmFileUpload { .. });
                (Task::none(), close.then_some(Event::CloseModal))
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
                    if path.is_some()
                        && let Modal::ImagePreview { timer, .. } = self
                    {
                        *timer = Some(Instant::now());
                    }

                    (Task::none(), None)
                }
                ImagePreview::Animation(message) => {
                    if let Modal::ImagePreview {
                        animation: Some(animation),
                        ..
                    } = self
                    {
                        animation.update(message);
                    }
                    (Task::none(), None)
                }
            },
            Message::KeyringPassword(action) => {
                if let Modal::KeyringPassword(keyring_password) = self {
                    keyring_password.update(action)
                } else {
                    (Task::none(), None)
                }
            }
        }
    }

    pub fn view<'a>(
        &'a self,
        font: &'a config::Font,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        match self {
            Modal::ReloadConfigurationError(error) => {
                reload_configuration_error::view(error, font, theme)
            }
            Modal::ServerConnect {
                url: raw, config, ..
            } => connect_to_server::view(raw, config, font, theme),
            Modal::About(about) => about.view(font, theme),
            Modal::PromptBeforeOpenUrl { url, window: _ } => {
                prompt_before_open_url::view(url, font, theme)
            }
            Modal::ConfirmFileUpload {
                url,
                has_credentials,
                window: _,
            } => confirm_file_upload::view(url, *has_credentials, font, theme),
            Modal::ImagePreview {
                image,
                timer,
                animation,
                window: _,
            } => image_preview::view(image, timer, animation.as_ref(), theme),
            Modal::KeyringPassword(keyring_password) => {
                keyring_password.view(font, theme)
            }
        }
    }
}
