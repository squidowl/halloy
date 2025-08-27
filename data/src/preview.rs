use std::collections::HashMap;
use std::io;
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;

use ::image::image_dimensions;
use fancy_regex::Regex;
use iced_wgpu::wgpu;
use log::debug;
use reqwest::header::{self, HeaderValue};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tokio::time;
use url::Url;

pub use self::card::Card;
pub use self::image::Image;
use crate::target::Target;
use crate::{config, isupport};

mod cache;
pub mod card;
pub mod image;

// Prevent us from rate limiting ourselves
static RATE_LIMIT: OnceLock<Semaphore> = OnceLock::new();
static CLIENT: LazyLock<reqwest::Client> =
    LazyLock::new(|| reqwest::Client::builder().build().expect("build client"));
static OPENGRAPH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?m)<meta[^>]+(name|property|content)=("[^"]+"|'[^']+')[^>]+(name|property|content)=("[^"]+"|'[^']+')[^>]*\/?>"#,
    )
    .expect("valid opengraph regex")
});

#[derive(Clone, Copy)]
pub struct Previews<'a> {
    collection: &'a Collection,
    cards_are_visible: bool,
    images_are_visible: bool,
}

impl<'a> Previews<'a> {
    pub fn new(
        collection: &'a Collection,
        target: &Target,
        config: &config::Preview,
        casemapping: isupport::CaseMap,
    ) -> Previews<'a> {
        Self {
            collection,
            cards_are_visible: config.enabled
                && config.card.visible(target, casemapping),
            images_are_visible: config.enabled
                && config.image.visible(target, casemapping),
        }
    }

    pub fn get(&self, url: &Url) -> Option<&'a State> {
        self.collection.get(url).filter(|state| match state {
            State::Loading => true,
            State::Loaded(preview) => match preview {
                Preview::Card(_) => self.cards_are_visible,
                Preview::Image(_) => self.images_are_visible,
            },
            State::Error(_) => true,
        })
    }
}

pub type Collection = HashMap<Url, State>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preview {
    Card(Card),
    Image(Image),
}

impl Preview {
    pub fn image(&self) -> &Image {
        match self {
            Self::Card(card) => &card.image,
            Self::Image(image) => image,
        }
    }
}

#[derive(Debug)]
pub enum State {
    Loading,
    Loaded(Preview),
    Error(LoadError),
}

pub async fn load(
    url: Url,
    config: config::Preview,
) -> Result<Preview, LoadError> {
    if !config.enabled {
        return Err(LoadError::Disabled);
    }

    let result = if let Some(state) = cache::load(&url, &config).await {
        match state {
            cache::State::Ok(preview) => Ok(preview),
            cache::State::Error => Err(LoadError::CachedFailed),
        }
    } else {
        match load_uncached(url.clone(), &config).await {
            Ok(preview) => {
                cache::save(&url, cache::State::Ok(preview.clone())).await;

                Ok(preview)
            }
            Err(error) => {
                cache::save(&url, cache::State::Error).await;

                Err(error)
            }
        }
    };

    if let Ok(ref preview) = result {
        let image = preview.image();

        if let Ok((image_width, image_height)) = image_dimensions(&image.path) {
            // As per iced, it is a webgpu requirement that:
            //   BufferCopyView.layout.bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT == 0
            // So we calculate padded_width by rounding width up to the next
            // multiple of wgpu::COPY_BYTES_PER_ROW_ALIGNMENT.
            let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let padding = (align - (4 * image_width) % align) % align;
            let padded_image_width = (4 * image_width + padding) as u64;
            let padded_image_data_size =
                padded_image_width * image_height as u64;

            let max_buffer_size =
                wgpu::Limits::downlevel_defaults().max_buffer_size;

            if padded_image_data_size > max_buffer_size {
                Err(LoadError::ImageDimensionsTooLarge {
                    padded_image_data_size,
                    max_buffer_size,
                })
            } else {
                result
            }
        } else {
            Err(LoadError::ImageDimensionsUnknown)
        }
    } else {
        result
    }
}

async fn load_uncached(
    url: Url,
    config: &config::Preview,
) -> Result<Preview, LoadError> {
    debug!("Loading preview for {url}");

    match fetch(url.clone(), config).await? {
        Fetched::Image(image) => Ok(Preview::Image(image)),
        Fetched::Other(bytes) => {
            let mut canonical_url = None;
            let mut image_url = None;
            let mut title = None;
            let mut description = None;

            for captures in OPENGRAPH_REGEX
                .captures_iter(&String::from_utf8_lossy(&bytes))
                .filter_map(Result::ok)
            {
                let Some((((key_1, value_1), key_2), value_2)) = captures
                    .get(1)
                    .map(|r| r.as_str())
                    .zip(captures.get(2).map(|r| r.as_str()))
                    .zip(captures.get(3).map(|r| r.as_str()))
                    .zip(captures.get(4).map(|r| r.as_str()))
                else {
                    continue;
                };

                let value_1 = decode_html_string(
                    value_1
                        .trim_start_matches(['\'', '"'])
                        .trim_end_matches(['\'', '"']),
                );
                let value_2 = decode_html_string(
                    value_2
                        .trim_start_matches(['\'', '"'])
                        .trim_end_matches(['\'', '"']),
                );

                let (property, content) = if (key_1 == "property"
                    || key_1 == "name")
                    && key_2 == "content"
                {
                    (value_1, value_2)
                } else if key_1 == "content"
                    && (key_2 == "property" || key_2 == "name")
                {
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

            let image_url =
                image_url.ok_or(LoadError::MissingProperty("image"))?;

            let Fetched::Image(image) = fetch(image_url, config).await? else {
                return Err(LoadError::NotImage);
            };

            Ok(Preview::Card(Card {
                url: url.clone(),
                canonical_url: canonical_url
                    .ok_or(LoadError::MissingProperty("url"))?,
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

async fn fetch(
    url: Url,
    config: &config::Preview,
) -> Result<Fetched, LoadError> {
    // WARN: `concurrency` changes aren't picked up until app is relaunchd
    let _permit = RATE_LIMIT
        .get_or_init(|| Semaphore::new(config.request.concurrency))
        .acquire()
        .await;

    let mut req = CLIENT
        .get(url.clone())
        .timeout(Duration::from_millis(config.request.timeout_ms));

    if let Ok(user_agent) = HeaderValue::from_str(&config.request.user_agent) {
        req = req.header(header::USER_AGENT, user_agent);
    }

    let mut resp = req.send().await?.error_for_status()?;

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
                if written + chunk.len() > config.request.max_image_size {
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
            let max_scrape_size = config.request.max_scrape_size;

            let mut buffer = Vec::with_capacity(max_scrape_size);
            buffer.extend(first_chunk);

            while let Some(mut chunk) = resp.chunk().await? {
                if buffer.len() + chunk.len() > max_scrape_size {
                    buffer.extend(chunk.split_to(
                        max_scrape_size.saturating_sub(buffer.len()),
                    ));
                    break;
                } else {
                    buffer.extend(chunk);
                }
            }

            Fetched::Other(buffer)
        }
    };

    // Artificially wait before releasing this permit for rate limiting
    time::sleep(Duration::from_millis(config.request.delay_ms)).await;

    Ok(fetched)
}

fn decode_html_string(s: &str) -> String {
    html_escape::decode_html_entities(s).to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("previews disabled in config")]
    Disabled,
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
    #[error("unable to verify image dimensions fit in maximum buffer size")]
    ImageDimensionsUnknown,
    #[error(
        "image dimensions too large to fit in maximum buffer size ({padded_image_data_size} > {max_buffer_size})"
    )]
    ImageDimensionsTooLarge {
        padded_image_data_size: u64,
        max_buffer_size: u64,
    },
}
