use std::collections::HashMap;
use std::sync::Arc;

use iced::Task;
use reqwest_middleware::ClientWithMiddleware;
use url::Url;

use crate::Server;
use crate::image::{self, Image};

#[derive(Debug)]
pub enum Message {
    Loaded(Server, Url, Result<Image, LoadError>),
}

#[derive(Default)]
pub struct Manager {
    icons: HashMap<Server, Image>,
    pending: HashMap<Server, Url>,
}

impl Manager {
    pub fn request(
        &mut self,
        server: &Server,
        icon_url: Option<&str>,
        http_client: Option<Arc<ClientWithMiddleware>>,
    ) -> Task<Message> {
        let Some(icon_url) = icon_url else {
            self.drop_request(server);
            return Task::none();
        };

        let Ok(icon_url) = Url::parse(icon_url) else {
            log::debug!("invalid server icon URL for {server}: {icon_url}");
            self.drop_request(server);
            return Task::none();
        };

        let Some(http_client) = http_client else {
            log::warn!("server icon fetching disabled for {server}");
            self.drop_request(server);
            return Task::none();
        };

        if self
            .icons
            .get(server)
            .is_some_and(|icon| icon.url == icon_url)
            || self.pending.get(server) == Some(&icon_url)
        {
            return Task::none();
        }

        self.icons.remove(server);
        self.pending.insert(server.clone(), icon_url.clone());

        let server = server.clone();

        Task::perform(load(icon_url.clone(), http_client), move |result| {
            Message::Loaded(server, icon_url.clone(), result)
        })
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Loaded(server, icon_url, result) => {
                if self.pending.get(&server) != Some(&icon_url) {
                    log::trace!(
                        "ignoring stale server icon result for {server}: {icon_url}"
                    );
                    return;
                }

                self.pending.remove(&server);

                match result {
                    Ok(icon) => {
                        self.icons.insert(server, icon);
                    }
                    Err(error) => {
                        log::debug!(
                            "failed to load server icon for {server}: {error}"
                        );
                        self.icons.remove(&server);
                    }
                }
            }
        }
    }

    pub fn get(&self, server: &Server) -> Option<&Image> {
        self.icons.get(server)
    }

    fn drop_request(&mut self, server: &Server) {
        self.pending.remove(server);
        self.icons.remove(server);
    }
}

const MAX_ICON_SIZE: usize = 5 * 1024 * 1024; // 5 MiB

async fn load(
    url: Url,
    http_client: Arc<ClientWithMiddleware>,
) -> Result<Image, LoadError> {
    let mut resp = http_client
        .get(url.clone())
        .send()
        .await?
        .error_for_status()?;

    let Some(first_chunk) = resp.chunk().await? else {
        return Err(LoadError::EmptyBody);
    };

    // First chunk should always be enough bytes to detect raster image
    // MAGIC value (<32 bytes)
    let Some(format) = image::Format::from_magic_bytes(&first_chunk).or(resp
        .headers()
        .get("content-type")
        .and_then(|content_type| content_type.to_str().ok())
        .and_then(image::Format::from_mime_type))
    else {
        return Err(LoadError::ParseImage);
    };

    let mut bytes = Vec::new();

    bytes.extend_from_slice(&first_chunk);

    while let Some(chunk) = resp.chunk().await? {
        if bytes.len() + chunk.len() > MAX_ICON_SIZE {
            return Err(LoadError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(Image::new(format, url, bytes))
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("empty body")]
    EmptyBody,
    #[error("image too large")]
    TooLarge,
    #[error("failed to parse image")]
    ParseImage,
    #[error("request failed: {0}")]
    Http(#[from] reqwest_middleware::Error),
    #[error("request failed: {0}")]
    Reqwest(#[from] reqwest_middleware::reqwest::Error),
}
