use std::collections::HashMap;
use std::io;
use std::sync::{Arc, LazyLock, OnceLock};
use std::time::Duration;

use ::image::image_dimensions;
use fancy_regex::Regex;
use iced_wgpu::wgpu;
use log;
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
use crate::message::Source;
use crate::server::Server;
use crate::target::{self, Target};
use crate::{config, isupport};

mod cache;
pub mod card;
pub mod image;

// Prevent us from rate limiting ourselves
static RATE_LIMIT: OnceLock<Semaphore> = OnceLock::new();
static META_TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<meta\b[^>]*?>"#).expect("valid meta tag regex")
});
static META_ATTR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)\b([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#,
    )
    .expect("valid meta attribute regex")
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
        server: &Server,
        config: &config::Preview,
        casemapping: isupport::CaseMap,
    ) -> Previews<'a> {
        Self {
            collection,
            cards_are_visible: config.card.visible(target, server, casemapping),
            images_are_visible: config.image.visible(
                target,
                server,
                casemapping,
            ),
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

    pub fn visible_for_source(
        &self,
        source: &Source,
        channel: Option<&target::Channel>,
        server: Option<&Server>,
        casemapping: isupport::CaseMap,
        config: &config::Preview,
    ) -> bool {
        match self {
            Self::Card(_) => config.card.visible_for_source(
                source,
                channel,
                server,
                casemapping,
            ),
            Self::Image(_) => config.image.visible_for_source(
                source,
                channel,
                server,
                casemapping,
            ),
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
    client: Arc<reqwest::Client>,
    config: config::Preview,
) -> Result<Preview, LoadError> {
    let cache_key_url = canonical_preview_url(&url);

    if !config.is_enabled(url.as_str()) {
        return Err(LoadError::Disabled);
    }

    let result = if let Some(state) =
        cache::load(&cache_key_url, client.clone(), &config).await
    {
        match state {
            cache::State::Ok(preview) => Ok(preview),
            cache::State::Error => Err(LoadError::CachedFailed),
        }
    } else {
        match load_uncached(url.clone(), client, &config).await {
            Ok(preview) => {
                cache::save(&cache_key_url, cache::State::Ok(preview.clone()))
                    .await;

                Ok(preview)
            }
            Err(error) => {
                cache::save(&cache_key_url, cache::State::Error).await;

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
            let padded_image_width = u64::from(4 * image_width + padding);
            let padded_image_data_size =
                padded_image_width * u64::from(image_height);

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

fn canonical_preview_url(url: &Url) -> Url {
    let mut canonical = url.clone();
    canonical.set_fragment(None);
    canonical
}

async fn load_uncached(
    url: Url,
    client: Arc<reqwest::Client>,
    config: &config::Preview,
) -> Result<Preview, LoadError> {
    log::trace!("Loading preview for {url}");

    match fetch(url.clone(), client.clone(), config).await? {
        Fetched::Image(image) => Ok(Preview::Image(image)),
        Fetched::Other(bytes) => {
            let MetaTagProperties {
                canonical_url,
                image_url,
                title,
                description,
            } = parse_meta_tag_properties(&bytes)?;

            let image_url =
                image_url.ok_or(LoadError::MissingProperty("image"))?;

            let Fetched::Image(image) =
                fetch(image_url, client, config).await?
            else {
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
    client: Arc<reqwest::Client>,
    config: &config::Preview,
) -> Result<Fetched, LoadError> {
    // WARN: `concurrency` changes aren't picked up until app is relaunched
    let _permit = RATE_LIMIT
        .get_or_init(|| Semaphore::new(config.request.concurrency))
        .acquire()
        .await;

    let mut req = client
        .get(url.clone())
        .timeout(Duration::from_millis(config.request.timeout_ms));

    if let Ok(user_agent) = HeaderValue::from_str(&config.request.user_agent) {
        req = req.header(header::USER_AGENT, user_agent);
    }

    let mut resp = req.send().await?.error_for_status()?;

    let Some(first_chunk) = resp.chunk().await? else {
        return Err(LoadError::EmptyBody);
    };

    // First chunk should always be enough bytes to detect image MAGIC value
    // (<32 bytes)
    let fetched = match image::format(&first_chunk) {
        Some(format) => {
            // Store image to disk, we don't want to explode memory
            let temp_path = cache::download_path(&url);

            if let Some(parent) = temp_path.parent().filter(|p| !p.exists()) {
                fs::create_dir_all(&parent).await?;
            }

            let image_result = async {
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

                if let Some(parent) =
                    image_path.parent().filter(|p| !p.exists())
                {
                    fs::create_dir_all(&parent).await?;
                }

                fs::rename(&temp_path, &image_path).await?;

                Ok::<Image, LoadError>(Image::new(format, url, digest))
            }
            .await;

            if image_result.is_err() {
                remove_download_file(&temp_path).await;
            }

            Fetched::Image(image_result?)
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

async fn remove_download_file(path: &std::path::Path) {
    let _ = fs::remove_file(path).await;
}

fn decode_html_string(s: &str) -> String {
    html_escape::decode_html_entities(s).to_string()
}

#[derive(Debug, Default)]
struct MetaTagProperties {
    canonical_url: Option<Url>,
    image_url: Option<Url>,
    title: Option<String>,
    description: Option<String>,
}

fn parse_meta_tag_properties(
    bytes: &[u8],
) -> Result<MetaTagProperties, LoadError> {
    let mut meta = MetaTagProperties::default();

    for meta_tag in META_TAG_REGEX
        .find_iter(&String::from_utf8_lossy(bytes))
        .filter_map(Result::ok)
    {
        let meta_tag = meta_tag.as_str();
        let mut property = None;
        let mut content = None;

        for captures in META_ATTR_REGEX
            .captures_iter(meta_tag)
            .filter_map(Result::ok)
        {
            let Some((key, value)) = captures
                .get(1)
                .map(|r| r.as_str())
                .zip(captures.get(2).map(|r| r.as_str()))
            else {
                continue;
            };

            let key = key.trim().to_ascii_lowercase();
            let value = decode_html_string(
                value
                    .trim_start_matches(['\'', '"'])
                    .trim_end_matches(['\'', '"']),
            )
            .trim()
            .to_string();

            match key.as_str() {
                "property" => property = Some(value),
                "name" => {
                    if property.is_none() {
                        property = Some(value);
                    }
                }
                "content" => content = Some(value),
                _ => {}
            }
        }

        let (Some(property), Some(content)) = (property, content) else {
            continue;
        };

        match property.trim().to_ascii_lowercase().as_str() {
            "og:url" => {
                if meta.canonical_url.is_none() {
                    meta.canonical_url = Some(content.parse()?);
                }
            }
            "og:image" | "og:image:url" | "og:image:secure_url" => {
                if meta.image_url.is_none() {
                    meta.image_url = Some(content.parse()?);
                }
            }
            "og:title" => {
                if meta.title.is_none() {
                    meta.title = Some(content);
                }
            }
            "og:description" => {
                if meta.description.is_none() {
                    meta.description = Some(content);
                }
            }
            _ => {}
        }
    }

    Ok(meta)
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("preview disabled in config")]
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

#[cfg(test)]
mod tests {
    use super::{canonical_preview_url, parse_meta_tag_properties};

    #[test]
    fn canonical_preview_url_strips_fragment_but_keeps_query() {
        let first: url::Url = "https://example.com/image.jpg?x=1#a"
            .parse()
            .expect("valid URL");
        let second: url::Url = "https://example.com/image.jpg?x=1#b"
            .parse()
            .expect("valid URL");

        assert_eq!(
            canonical_preview_url(&first),
            canonical_preview_url(&second)
        );
    }

    #[test]
    fn parses_mixed_attribute_order_and_quotes() {
        let html = br#"
            <html><head>
                <meta content="https://example.com/page" property="og:url">
                <meta property='og:image' content='https://cdn.example.com/a.png'>
                <meta content="Title" property="og:title">
                <meta property="og:description" content="  Hello &amp; goodbye  ">
            </head></html>
        "#;

        let meta = parse_meta_tag_properties(html).expect("should parse");

        assert_eq!(
            meta.canonical_url.as_ref().map(url::Url::as_str),
            Some("https://example.com/page")
        );
        assert_eq!(
            meta.image_url.as_ref().map(url::Url::as_str),
            Some("https://cdn.example.com/a.png")
        );
        assert_eq!(meta.title.as_deref(), Some("Title"));
        assert_eq!(meta.description.as_deref(), Some("Hello & goodbye"));
    }

    #[test]
    fn parses_name_attr_and_secure_image_variant() {
        let html = br#"
            <meta name="og:image:secure_url" content="https://img.example.com/secure.jpg">
            <meta name="og:title" content="From name attr">
            <meta name="og:url" content="https://example.com/post">
        "#;

        let meta = parse_meta_tag_properties(html).expect("should parse");

        assert_eq!(
            meta.image_url.as_ref().map(url::Url::as_str),
            Some("https://img.example.com/secure.jpg")
        );
        assert_eq!(meta.title.as_deref(), Some("From name attr"));
        assert_eq!(
            meta.canonical_url.as_ref().map(url::Url::as_str),
            Some("https://example.com/post")
        );
    }

    #[test]
    fn first_value_wins_for_duplicates() {
        let html = br#"
            <meta property="og:title" content="First">
            <meta property="og:title" content="Second">
            <meta property="og:url" content="https://example.com/one">
            <meta property="og:url" content="https://example.com/two">
            <meta property="og:image" content="https://example.com/img1.png">
            <meta property="og:image" content="https://example.com/img2.png">
        "#;

        let meta = parse_meta_tag_properties(html).expect("should parse");

        assert_eq!(meta.title.as_deref(), Some("First"));
        assert_eq!(
            meta.canonical_url.as_ref().map(url::Url::as_str),
            Some("https://example.com/one")
        );
        assert_eq!(
            meta.image_url.as_ref().map(url::Url::as_str),
            Some("https://example.com/img1.png")
        );
    }

    #[test]
    fn property_attribute_takes_precedence_over_name_on_same_meta_tag() {
        let html = br#"
            <meta property="og:image" name="twitter:image" content="https://example.com/og.png">
        "#;

        let meta = parse_meta_tag_properties(html).expect("should parse");

        assert_eq!(
            meta.image_url.as_ref().map(url::Url::as_str),
            Some("https://example.com/og.png")
        );
    }
}
