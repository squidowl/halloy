use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use iced::Task;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio_stream::StreamExt;
use url::Url;

use self::icon::Icon;
use crate::cache::{self, Asset, CacheState, CachedAsset, FileCache};
use crate::{Server, environment};

mod icon;

#[derive(Debug)]
pub enum Message {
    Loaded(Server, Url, Result<Icon, LoadError>),
}

pub struct Manager {
    icons: HashMap<Server, Icon>,
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
                "[{server}] File upload disabled: Unable to build HTTP client"
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

        Task::perform(
            load(icon_url.clone(), http_client, self.cache.clone()),
            move |result| {
                Message::Loaded(server.clone(), icon_url.clone(), result)
            },
        )
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

impl CachedAsset for Icon {
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
) -> Result<Icon, LoadError> {
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
                cache.save::<Icon>(&cache_key_url, &CacheState::Error).await;

                Err(error)
            }
        }
    }
}

const MAX_ICON_SIZE: usize = 5 * 1024 * 1024; // 5 MiB

async fn fetch(
    url: Url,
    http_client: Arc<reqwest::Client>,
    cache: &FileCache,
) -> Result<Icon, LoadError> {
    let mut stream = http_client
        .get(url.clone())
        .send()
        .await?
        .error_for_status()?
        .bytes_stream();

    let mut bytes = Vec::new();

    while let Some(chunk) = stream.next().await.transpose()? {
        if bytes.len() + chunk.len() > MAX_ICON_SIZE {
            return Err(LoadError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }

    if bytes.is_empty() {
        return Err(LoadError::EmptyBody);
    }

    let format = image::guess_format(&bytes).map_err(LoadError::ParseImage)?;

    let mut hasher = Sha256::default();
    hasher.update(&bytes);

    let digest = cache::HexDigest::new(&hasher.finalize());
    let image_path = cache.blob_path(&digest, format.extensions_str()[0]);

    if !image_path.exists() {
        if let Some(parent) = image_path.parent().filter(|p| !p.exists()) {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&image_path, &bytes).await?;

        cache.account_blob(bytes.len() as u64, image_path.clone());
    }

    Ok(Icon::new(format, url, digest, image_path))
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("cached failed attempt")]
    CachedFailed,
    #[error("empty body")]
    EmptyBody,
    #[error("image too large")]
    TooLarge,
    #[error("failed to parse image: {0}")]
    ParseImage(#[from] icon::Error),
    #[error("request failed: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}
