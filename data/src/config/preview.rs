use serde::Deserialize;

use crate::serde::default_bool_true;
use crate::{Target, isupport};

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
    /// for wide compatibility
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
    /// Number of milliseconds to wait before requesting another preview
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
            exclude: Vec::default(),
            include: Vec::default(),
            show_image: true,
        }
    }
}

impl Card {
    pub fn visible(
        &self,
        target: &Target,
        casemapping: isupport::CaseMap,
    ) -> bool {
        is_visible(&self.include, &self.exclude, target, casemapping)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Image {
    #[serde(default)]
    pub action: ImageAction,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ImageAction {
    OpenUrl,
    #[default]
    Preview,
}

impl Image {
    pub fn visible(
        &self,
        target: &Target,
        casemapping: isupport::CaseMap,
    ) -> bool {
        is_visible(&self.include, &self.exclude, target, casemapping)
    }
}

fn is_visible(
    include: &[String],
    exclude: &[String],
    target: &Target,
    casemapping: isupport::CaseMap,
) -> bool {
    let target = match target {
        Target::Query(user) => user.as_normalized_str(),
        Target::Channel(channel) => channel.as_normalized_str(),
    };

    let is_target_filtered = |list: &[String], target: &str| -> bool {
        let wildcards = ["*", "all"];
        list.iter().any(|item| {
            wildcards.contains(&item.as_str())
                || casemapping.normalize(item) == target
        })
    };

    let target_included = is_target_filtered(include, target);
    let target_excluded = is_target_filtered(exclude, target);

    target_included || !target_excluded
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
