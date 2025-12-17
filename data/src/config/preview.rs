use fancy_regex::{Regex, RegexBuilder};
use serde::{Deserialize, Deserializer};

use crate::config::inclusivities::{
    Inclusivities, is_source_included, is_target_included,
};
use crate::message::Source;
use crate::{Server, Target, isupport, target};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Preview {
    pub enabled: Enabled,
    pub request: Request,
    pub card: Card,
    pub image: Image,
}

impl Preview {
    pub fn is_enabled(&self, url: &str) -> bool {
        match &self.enabled {
            Enabled::Boolean(b) => *b,
            Enabled::Regex(regexes) => regexes
                .iter()
                .any(|regex| regex.is_match(url).unwrap_or(false)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Enabled {
    Boolean(bool),
    Regex(Vec<Regex>),
}

impl Default for Enabled {
    fn default() -> Self {
        Self::Boolean(true)
    }
}

impl<'de> Deserialize<'de> for Enabled {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        pub enum Inner {
            Boolean(bool),
            RegexSingle(String),
            RegexMultiple(Vec<String>),
        }

        match Inner::deserialize(deserializer)? {
            Inner::Boolean(enabled) => Ok(Enabled::Boolean(enabled)),
            Inner::RegexSingle(regex_str) => {
                let regex =
                    RegexBuilder::new(&regex_str).build().map_err(|err| {
                        serde::de::Error::custom(format!(
                            "invalid regex '{regex_str}': {err}"
                        ))
                    })?;

                Ok(Enabled::Regex(vec![regex]))
            }
            Inner::RegexMultiple(regex_strs) => {
                let regexes = regex_strs
                    .iter()
                    .map(|regex_str| {
                        RegexBuilder::new(regex_str).build().map_err(|err| {
                            serde::de::Error::custom(format!(
                                "invalid regex '{regex_str}': {err}"
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Enabled::Regex(regexes))
            }
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
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
    pub show_image: bool,
    pub round_image_corners: bool,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            exclude: None,
            include: None,
            show_image: true,
            round_image_corners: true,
        }
    }
}

impl Card {
    pub fn visible(
        &self,
        target: &Target,
        server: &Server,
        casemapping: isupport::CaseMap,
    ) -> bool {
        is_target_included(
            self.include.as_ref(),
            self.exclude.as_ref(),
            None,
            target,
            server,
            casemapping,
        )
    }

    pub fn visible_for_source(
        &self,
        source: &Source,
        channel: Option<&target::Channel>,
        server: Option<&Server>,
        casemapping: isupport::CaseMap,
    ) -> bool {
        is_source_included(
            self.include.as_ref(),
            self.exclude.as_ref(),
            source,
            channel,
            server,
            casemapping,
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Image {
    pub action: ImageAction,
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
    pub round_corners: bool,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            action: ImageAction::default(),
            exclude: None,
            include: None,
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
        server: &Server,
        casemapping: isupport::CaseMap,
    ) -> bool {
        is_target_included(
            self.include.as_ref(),
            self.exclude.as_ref(),
            None,
            target,
            server,
            casemapping,
        )
    }

    pub fn visible_for_source(
        &self,
        source: &Source,
        channel: Option<&target::Channel>,
        server: Option<&Server>,
        casemapping: isupport::CaseMap,
    ) -> bool {
        is_source_included(
            self.include.as_ref(),
            self.exclude.as_ref(),
            source,
            channel,
            server,
            casemapping,
        )
    }
}
