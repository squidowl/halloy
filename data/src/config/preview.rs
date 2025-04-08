use serde::Deserialize;

use crate::serde::default_bool_true;
use crate::Target;

#[derive(Debug, Clone, Deserialize)]
pub struct Preview {
    #[serde(default = "default_bool_true")]
    pub enabled: bool,
    #[serde(default)]
    pub request: Request,
    #[serde(default)]
    pub card: Card,
    #[serde(default)]
    pub image: Image,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            enabled: default_bool_true(),
            request: Request::default(),
            card: Card::default(),
            image: Image::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    /// Request user agent
    ///
    /// Some servers will only send opengraph metadata to
    /// browser-like user agents. We default to `WhatsApp/2`
    /// for wide compatability
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    /// Request timeout in millisceonds
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Max image size in bytes
    ///
    /// This prevents downloading images that are too big
    #[serde(default = "default_max_image_size")]
    pub max_image_size: usize,
    /// Max bytes streamed when scraping for opengraph metadata
    /// before cancelling the request
    ///
    /// This prevents downloading responses that are too big
    #[serde(default = "default_max_scrape_size")]
    pub max_scrape_size: usize,
    /// Number of allowed concurrent requests for fetching previews
    ///
    /// Reduce this to prevent rate-limiting
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Numer of milliseconds to wait before requesting another preview
    /// when number of requested previews > `concurrency`
    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            user_agent: default_user_agent(),
            timeout_ms: default_timeout_ms(),
            max_image_size: default_max_image_size(),
            max_scrape_size: default_max_scrape_size(),
            concurrency: default_concurrency(),
            delay_ms: default_delay_ms(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Card {
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default = "default_bool_true")]
    pub show_image: bool,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            exclude: Default::default(),
            include: Default::default(),
            show_image: true,
        }
    }
}

impl Card {
    pub fn visible(&self, target: &Target) -> bool {
        is_visible(&self.include, &self.exclude, target)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Image {
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

impl Image {
    pub fn visible(&self, target: &Target) -> bool {
        is_visible(&self.include, &self.exclude, target)
    }
}

fn is_visible(include: &[String], exclude: &[String], target: &Target) -> bool {
    match target {
        Target::Query(_) => true,
        Target::Channel(channel) => {
            let is_channel_filtered = |list: &[String], channel: &str| -> bool {
                let wildcards = ["*", "all"];
                list.iter()
                    .any(|item| wildcards.contains(&item.as_str()) || item == channel)
            };

            let channel_included = is_channel_filtered(include, channel.as_str());
            let channel_excluded = is_channel_filtered(exclude, channel.as_str());

            channel_included || !channel_excluded
        }
    }
}

fn default_user_agent() -> String {
    "WhatsApp/2".to_string()
}

fn default_timeout_ms() -> u64 {
    10 * 1_000
}

/// 10 mb
fn default_max_image_size() -> usize {
    10 * 1024 * 1024
}

// 500 kb
fn default_max_scrape_size() -> usize {
    500 * 1024
}

fn default_concurrency() -> usize {
    4
}

fn default_delay_ms() -> u64 {
    500
}
