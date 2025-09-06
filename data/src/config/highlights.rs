use fancy_regex::{Regex, RegexBuilder};
use itertools::Itertools;
use serde::{Deserialize, Deserializer};

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
    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub case_insensitive: bool,
}

impl Default for Nickname {
    fn default() -> Self {
        Self {
            exclude: Vec::default(),
            include: Vec::default(),
            case_insensitive: true,
        }
    }
}

impl Nickname {
    pub fn is_target_included(&self, target: &str) -> bool {
        is_target_included(&self.include, &self.exclude, target)
    }
}

#[derive(Debug, Clone)]
pub struct Match {
    pub regex: Regex,
    pub exclude: Vec<String>,
    pub include: Vec<String>,
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
                exclude: Vec<String>,
                #[serde(default)]
                include: Vec<String>,
                #[serde(default)]
                case_insensitive: bool,
            },
            Regex {
                regex: String,
                #[serde(default)]
                exclude: Vec<String>,
                #[serde(default)]
                include: Vec<String>,
            },
        }

        match Inner::deserialize(deserializer)? {
            Inner::Words {
                words,
                exclude,
                include,
                case_insensitive,
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
                })
            }
            Inner::Regex {
                regex,
                exclude,
                include,
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
                })
            }
        }
    }
}

impl Match {
    pub fn is_target_included(&self, target: &str) -> bool {
        is_target_included(&self.include, &self.exclude, target)
    }
}

fn is_target_included(
    include: &[String],
    exclude: &[String],
    target: &str,
) -> bool {
    let is_channel_filtered = |list: &[String], target: &str| -> bool {
        let wildcards = ["*", "all"];
        list.iter()
            .any(|item| wildcards.contains(&item.as_str()) || item == target)
    };

    let channel_included = is_channel_filtered(include, target);
    let channel_excluded = is_channel_filtered(exclude, target);

    channel_included || !channel_excluded
}
