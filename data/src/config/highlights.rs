use fancy_regex::{Regex, RegexBuilder};
use itertools::Itertools;
use serde::{Deserialize, Deserializer};

use crate::config::inclusivities::{Inclusivities, is_target_included};
use crate::target::Target;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Highlights {
    pub nickname: Nickname,
    #[serde(rename = "match")]
    pub matches: Vec<Match>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Nickname {
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
    pub case_insensitive: bool,
}

impl Default for Nickname {
    fn default() -> Self {
        Self {
            exclude: None,
            include: None,
            case_insensitive: true,
        }
    }
}

impl Nickname {
    pub fn is_target_included(&self, target: &Target) -> bool {
        is_target_included(self.include.as_ref(), self.exclude.as_ref(), target)
    }
}

#[derive(Debug, Clone)]
pub struct Match {
    pub regex: Regex,
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
    pub sound: Option<String>,
}

impl<'de> Deserialize<'de> for Match {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(rename_all = "kebab-case", untagged)]
        pub enum Inner {
            Words {
                words: Vec<String>,
                #[serde(default)]
                exclude: Option<Inclusivities>,
                #[serde(default)]
                include: Option<Inclusivities>,
                #[serde(default)]
                case_insensitive: bool,
                #[serde(default)]
                sound: Option<String>,
            },
            Regex {
                regex: String,
                #[serde(default)]
                exclude: Option<Inclusivities>,
                #[serde(default)]
                include: Option<Inclusivities>,
                #[serde(default)]
                sound: Option<String>,
            },
        }

        match Inner::deserialize(deserializer)? {
            Inner::Words {
                words,
                exclude,
                include,
                case_insensitive,
                sound,
            } => {
                let words =
                    words.iter().map(|s| fancy_regex::escape(s)).join("|");

                let flags = if case_insensitive { "(?i)" } else { "" };

                let regex = format!(r#"{flags}(?<!\w)({words})(?!\w)"#);

                let regex =
                    RegexBuilder::new(&regex).build().map_err(|err| {
                        serde::de::Error::custom(format!(
                            "invalid regex '{regex}': {err}"
                        ))
                    })?;

                Ok(Match {
                    regex,
                    exclude,
                    include,
                    sound,
                })
            }
            Inner::Regex {
                regex,
                exclude,
                include,
                sound,
            } => {
                let regex =
                    RegexBuilder::new(&regex).build().map_err(|err| {
                        serde::de::Error::custom(format!(
                            "invalid regex '{regex}': {err}"
                        ))
                    })?;

                Ok(Match {
                    regex,
                    exclude,
                    include,
                    sound,
                })
            }
        }
    }
}

impl Match {
    pub fn is_target_included(&self, target: &Target) -> bool {
        is_target_included(self.include.as_ref(), self.exclude.as_ref(), target)
    }
}
