use serde::Deserialize;

use crate::{
    Target, isupport,
    message::{Kind, Source},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Preview {
    pub enabled: bool,
    pub request: Request,
    pub card: Card,
    pub image: Image,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            enabled: true,
            request: Request::default(),
            card: Card::default(),
            image: Image::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Request {
    /// Request user agent
    ///
    /// Some servers will only send opengraph metadata to
    /// browser-like user agents. We default to `WhatsApp/2`
    /// for wide compatibility
    pub user_agent: String,
    /// Request timeout in millisceonds
    pub timeout_ms: u64,
    /// Max image size in bytes
    ///
    /// This prevents downloading images that are too big
    pub max_image_size: usize,
    /// Max bytes streamed when scraping for opengraph metadata
    /// before cancelling the request
    ///
    /// This prevents downloading responses that are too big
    pub max_scrape_size: usize,
    /// Number of allowed concurrent requests for fetching previews
    ///
    /// Reduce this to prevent rate-limiting
    pub concurrency: usize,
    /// Number of milliseconds to wait before requesting another preview
    /// when number of requested previews > `concurrency`
    pub delay_ms: u64,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            user_agent: "WhatsApp/2".to_string(),
            timeout_ms: 10 * 1_000,
            max_image_size: 10 * 1024 * 1024,
            max_scrape_size: 500 * 1024,
            concurrency: 4,
            delay_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Card {
    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub show_image: bool,
    pub round_image_corners: bool,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            exclude: Vec::default(),
            include: Vec::default(),
            show_image: true,
            round_image_corners: true,
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

    pub fn visible_for_source(&self, source: &Source) -> bool {
        is_visible_for_source(&self.exclude, source)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Image {
    pub action: ImageAction,
    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub round_corners: bool,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            action: ImageAction::default(),
            exclude: Vec::default(),
            include: Vec::default(),
            round_corners: true,
        }
    }
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

    pub fn visible_for_source(&self, source: &Source) -> bool {
        is_visible_for_source(&self.exclude, source)
    }
}

pub fn is_visible_for_source(exclude: &[String], source: &Source) -> bool {
    if let Source::Server(Some(server)) = source {
        let kind = server.kind();
        return !exclude.iter().any(|item| match item.to_lowercase().as_str() {
            "topic" => matches!(kind, Kind::ReplyTopic | Kind::ChangeTopic),
            "part" => matches!(kind, Kind::Part),
            "quit" => matches!(kind, Kind::Quit),
            _ => false,
        });
    }
    true
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
