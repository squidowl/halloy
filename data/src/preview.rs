use std::{collections::HashMap, io, sync::LazyLock, time::Duration};

use log::debug;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    sync::Semaphore,
    time,
};
use url::Url;

pub use self::card::Card;
pub use self::image::Image;

mod cache;
pub mod card;
pub mod image;

// TODO: Make these configurable at request level
const TIMEOUT: Duration = Duration::from_secs(10);
const RATE_LIMIT_DELAY: Duration = Duration::from_millis(100);

// Prevent us from rate limiting ourselves
static RATE_LIMIT: Semaphore = Semaphore::const_new(4);
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent("halloy")
        .timeout(TIMEOUT)
        .build()
        .expect("build client")
});
static OPENGRAPH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?m)<meta[^>]+(name|property|content)=("[^"]+"|'[^']+')[^>]+(name|property|content)=("[^"]+"|'[^']+')[^>]*\/?>"#,
    )
    .expect("valid opengraph regex")
});

pub type Collection = HashMap<Url, State>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preview {
    Card(Card),
    Image(Image),
}

#[derive(Debug)]
pub enum State {
    Loading,
    Loaded(Preview),
    Error(LoadError),
}

pub async fn load(url: Url) -> Result<Preview, LoadError> {
    if let Some(state) = cache::load(&url).await {
        match state {
            cache::State::Ok(preview) => return Ok(preview),
            cache::State::Error => return Err(LoadError::CachedFailed),
        }
    }

    match load_uncached(url.clone()).await {
        Ok(preview) => {
            cache::save(&url, cache::State::Ok(preview.clone())).await;

            Ok(preview)
        }
        Err(error) => {
            cache::save(&url, cache::State::Error).await;

            Err(error)
        }
    }
}

async fn load_uncached(url: Url) -> Result<Preview, LoadError> {
    debug!("Loading preview for {url}");

    match fetch(url.clone()).await? {
        Fetched::Image(image) => Ok(Preview::Image(image)),
        Fetched::Other(bytes) => {
            let mut canonical_url = None;
            let mut image_url = None;
            let mut title = None;
            let mut description = None;

            for (_, [key_1, value_1, key_2, value_2]) in OPENGRAPH_REGEX
                .captures_iter(&String::from_utf8_lossy(&bytes))
                .map(|c| c.extract())
            {
                let value_1 = unescape(
                    value_1
                        .trim_start_matches(['\'', '"'])
                        .trim_end_matches(['\'', '"']),
                );
                let value_2 = unescape(
                    value_2
                        .trim_start_matches(['\'', '"'])
                        .trim_end_matches(['\'', '"']),
                );

                let (property, content) =
                    if (key_1 == "property" || key_1 == "name") && key_2 == "content" {
                        (value_1, value_2)
                    } else if key_1 == "content" && (key_2 == "property" || key_2 == "name") {
                        (value_2, value_1)
                    } else {
                        continue;
                    };

                match property.as_str() {
                    "og:url" => canonical_url = Some(content.parse()?),
                    "og:image" => image_url = Some(content.parse()?),
                    "og:title" => title = Some(content),
                    "og:description" => description = Some(content),
                    _ => {}
                }
            }

            let image_url = image_url.ok_or(LoadError::MissingProperty("image"))?;

            let Fetched::Image(image) = fetch(image_url).await? else {
                return Err(LoadError::NotImage);
            };

            Ok(Preview::Card(Card {
                url: url.clone(),
                canonical_url: canonical_url.ok_or(LoadError::MissingProperty("url"))?,
                image,
                title: title.ok_or(LoadError::MissingProperty("title"))?,
                description,
            }))
        }
    }
}

enum Fetched {
    Image(Image),
    Other(Vec<u8>),
}

async fn fetch(url: Url) -> Result<Fetched, LoadError> {
    // TODO: Make these configurable
    // 10 mb
    const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
    // 500 kb
    const MAX_OTHER_SIZE: usize = 500 * 1024;

    let _permit = RATE_LIMIT.acquire().await;

    let mut resp = CLIENT.get(url.clone()).send().await?.error_for_status()?;

    let Some(first_chunk) = resp.chunk().await? else {
        return Err(LoadError::EmptyBody);
    };

    // First chunk should always be enough bytes to detect
    // image MAGIC value (<32 bytes)
    let fetched = match image::format(&first_chunk) {
        Some(format) => {
            // Store image to disk, we don't want to explode memory
            let temp_path = cache::download_path(&url);

            if let Some(parent) = temp_path.parent().filter(|p| !p.exists()) {
                fs::create_dir_all(&parent).await?;
            }

            let mut file = File::create(&temp_path).await?;
            let mut hasher = Sha256::default();

            file.write_all(&first_chunk).await?;
            hasher.update(&first_chunk);

            let mut written = first_chunk.len();

            while let Some(chunk) = resp.chunk().await? {
                if written + chunk.len() > MAX_IMAGE_SIZE {
                    return Err(LoadError::ImageTooLarge);
                }

                file.write_all(&chunk).await?;
                hasher.update(&chunk);

                written += chunk.len();
            }

            let digest = image::Digest::new(&hasher.finalize());
            let image_path = cache::image_path(&format, &digest);

            if let Some(parent) = image_path.parent().filter(|p| !p.exists()) {
                fs::create_dir_all(&parent).await?;
            }

            fs::rename(temp_path, &image_path).await?;

            Fetched::Image(Image::new(format, url, digest))
        }
        None => {
            let mut buffer = Vec::with_capacity(MAX_OTHER_SIZE);
            buffer.extend(first_chunk);

            while let Some(mut chunk) = resp.chunk().await? {
                if buffer.len() + chunk.len() > MAX_OTHER_SIZE {
                    buffer.extend(chunk.split_to(MAX_OTHER_SIZE.saturating_sub(buffer.len())));
                    break;
                } else {
                    buffer.extend(chunk);
                }
            }

            Fetched::Other(buffer)
        }
    };

    // Artifically wait before releasing this
    // RATE_LIMIT permit
    time::sleep(RATE_LIMIT_DELAY).await;

    Ok(fetched)
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#x27", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("cached failed attempt")]
    CachedFailed,
    #[error("url doesn't contain open graph data")]
    MissingOpenGraphData,
    #[error("empty body")]
    EmptyBody,
    #[error("url is not html")]
    NotHtml,
    #[error("url is not an image")]
    NotImage,
    #[error("image exceeds max file size")]
    ImageTooLarge,
    #[error("failed to parse image: {0}")]
    ParseImage(#[from] image::Error),
    #[error("missing required property {0}")]
    MissingProperty(&'static str),
    #[error("request failed: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("failed to parse url: {0}")]
    ParseUrl(#[from] url::ParseError),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}
