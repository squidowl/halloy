use std::collections::HashMap;
use std::sync::Arc;

use iced::Task;
use sha2::{Digest, Sha256};
use tokio::fs;
use url::Url;

use self::icon::Icon;
use crate::Server;

mod cache;
mod icon;

#[derive(Debug)]
pub enum Message {
    Loaded(Server, Url, Result<Icon, String>),
}

pub struct Manager {
    icons: HashMap<Server, Icon>,
    pending: HashMap<Server, Url>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            icons: HashMap::new(),
            pending: HashMap::new(),
        }
    }

    pub fn request(
        &mut self,
        server: Server,
        icon_url: Option<String>,
        http_client: Option<Arc<reqwest::Client>>,
    ) -> Task<Message> {
        let Some(icon_url) = icon_url else {
            self.drop_request(&server);
            return Task::none();
        };

        let Ok(icon_url) = Url::parse(&icon_url) else {
            log::debug!("invalid server icon URL for {server}: {icon_url}");
            self.drop_request(&server);
            return Task::none();
        };

        let Some(http_client) = http_client else {
            log::warn!(
                "[{}] File upload disabled: Unable to build HTTP client",
                server
            );
            self.drop_request(&server);
            return Task::none();
        };

        if self
            .icons
            .get(&server)
            .is_some_and(|icon| icon.url == icon_url)
            || self.pending.get(&server) == Some(&icon_url)
        {
            return Task::none();
        }

        self.icons.remove(&server);
        self.pending.insert(server.clone(), icon_url.clone());

        Task::perform(load(icon_url.clone(), http_client), move |result| {
            Message::Loaded(server.clone(), icon_url.clone(), result)
        })
    }

    pub fn update(&mut self, message: Message) {
        let Message::Loaded(server, icon_url, result) = message;

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
                log::debug!("failed to load server icon for {server}: {error}");
                self.icons.remove(&server);
            }
        }
    }

    pub fn get(&self, server: &Server) -> Option<&Icon> {
        self.icons.get(server)
    }

    fn drop_request(&mut self, server: &Server) {
        self.pending.remove(server);
        self.icons.remove(server);
    }
}

fn canonical_icon_url(url: &Url) -> Url {
    let mut canonical = url.clone();
    canonical.set_fragment(None);
    canonical
}

async fn load(
    url: Url,
    http_client: Arc<reqwest::Client>,
) -> Result<Icon, String> {
    let cache_key_url = canonical_icon_url(&url);

    if let Some(state) = cache::load(&cache_key_url, http_client.clone()).await
    {
        match state {
            cache::State::Ok(icon) => Ok(icon),
            cache::State::Error => Err("cached failed".to_string()),
        }
    } else {
        match fetch(url.clone(), http_client).await {
            Ok(icon) => {
                cache::save(&cache_key_url, cache::State::Ok(icon.clone()))
                    .await;

                Ok(icon)
            }
            Err(error) => {
                cache::save(&cache_key_url, cache::State::Error).await;

                Err(error)
            }
        }
    }
}

async fn fetch(
    url: Url,
    http_client: Arc<reqwest::Client>,
) -> Result<Icon, String> {
    let response = http_client
        .get(url.clone())
        .send()
        .await
        .map_err(|error| format!("request failed: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "request failed with status {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("failed to read response body: {error}"))?;

    let format = image::guess_format(&bytes)
        .map_err(|error| format!("unsupported image format: {error}"))?;

    if format != image::ImageFormat::Ico {
        return Err(format!("unsupported image format: {format:?}"));
    }

    let mut hasher = Sha256::default();
    hasher.update(bytes.as_ref());

    let digest = icon::Digest::new(hasher.finalize().as_ref());
    let image_path = cache::image_path(&digest);

    if !image_path.exists() {
        if let Some(parent) = image_path.parent().filter(|p| !p.exists()) {
            let _ = fs::create_dir_all(parent).await;
        }

        fs::write(&image_path, bytes.as_ref())
            .await
            .map_err(|error| {
                format!("failed to write icon cache file: {error}")
            })?;

        cache::maybe_trim_icon_cache(bytes.len() as u64, image_path.clone());
    }

    Ok(Icon::new(url, digest))
}
