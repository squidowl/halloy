use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use iced::Task;
use sha2::{Digest, Sha256};
use tokio::fs;
use url::Url;

use crate::cache::{self, Asset, CacheState, CachedAsset, FileCache};
use crate::image::{self, Image};
use crate::{Server, environment};

#[derive(Debug)]
pub enum Message {
    Loaded(Server, Url, Result<Image, LoadError>),
    Removed(Server),
}

pub struct Manager {
    icons: HashMap<Server, Image>,
    pending: HashMap<Server, Url>,
    cache: Arc<FileCache>,
}

impl Default for Manager {
    fn default() -> Self {
        Self {
            icons: HashMap::new(),
            pending: HashMap::new(),
            cache: Arc::new(Self::server_icon_cache()),
        }
    }
}

impl Manager {
    pub fn request(
        &mut self,
        server: &Server,
        icon_url: Option<&str>,
        http_client: Option<Arc<reqwest::Client>>,
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

        Task::perform(
            load(icon_url.clone(), http_client, self.cache.clone()),
            move |result| Message::Loaded(server, icon_url.clone(), result),
        )
    }

    pub fn remove(
        &mut self,
        server: &Server,
        icon_url: Option<&str>,
    ) -> Task<Message> {
        self.pending.remove(server);
        self.icons.remove(server);

        let Some(icon_url) = icon_url else {
            return Task::none();
        };

        let Ok(icon_url) = Url::parse(icon_url) else {
            log::debug!("invalid server icon URL for {server}: {icon_url}");
            return Task::none();
        };

        let server = server.clone();

        Task::perform(remove(icon_url.clone(), self.cache.clone()), move |()| {
            Message::Removed(server)
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
            Message::Removed(server) => {
                log::trace!("removed server icon for {server}");
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

    fn server_icon_cache() -> cache::FileCache {
        let root = environment::cache_dir().join("server_icons");

        // A fixed sized cache is used since we expect icons to be small.
        cache::FileCache::new(
            root,
            Some(50 * 1024 * 1024), // 50 MiB
            32,
        )
    }
}

impl CachedAsset for Image {
    fn assets(&self) -> Vec<Asset<'_>> {
        vec![Asset(self.path.as_path(), &self.digest)]
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
    cache: Arc<FileCache>,
) -> Result<Image, LoadError> {
    let cache_key_url = canonical_icon_url(&url);

    if let Some(state) = cache.load(&cache_key_url).await {
        match state {
            CacheState::Ok(icon) => Ok(icon),
            CacheState::Error => Err(LoadError::CachedFailed),
        }
    } else {
        match fetch(url.clone(), http_client, &cache).await {
            Ok(icon) => {
                cache
                    .save(&cache_key_url, &CacheState::Ok(icon.clone()))
                    .await;

                Ok(icon)
            }
            Err(error) => {
                cache
                    .save::<Image>(&cache_key_url, &CacheState::Error)
                    .await;

                Err(error)
            }
        }
    }
}

async fn remove(url: Url, cache: Arc<FileCache>) {
    let cache_key_url = canonical_icon_url(&url);

    cache.remove::<Image>(&cache_key_url).await;
}

const MAX_ICON_SIZE: usize = 5 * 1024 * 1024; // 5 MiB

async fn fetch(
    url: Url,
    http_client: Arc<reqwest::Client>,
    cache: &FileCache,
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

    let mut hasher = Sha256::default();
    hasher.update(&bytes);

    let digest = cache::HexDigest::new(&hasher.finalize());
    let image_path = cache.blob_path(&digest, format.extensions_str()[0]);

    if !fs::try_exists(&image_path).await? {
        if let Some(parent) = image_path.parent().filter(|p| !p.exists()) {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&image_path, &bytes).await?;

        cache.account_blob(bytes.len() as u64, image_path.clone());
    }

    Ok(Image::new(format, url, digest, image_path))
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("cached failed attempt")]
    CachedFailed,
    #[error("empty body")]
    EmptyBody,
    #[error("image too large")]
    TooLarge,
    #[error("failed to parse image")]
    ParseImage,
    #[error("request failed: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}
