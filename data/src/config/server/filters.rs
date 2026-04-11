use fancy_regex::{Regex, RegexBuilder};
use serde::{Deserialize, Deserializer};

#[derive(PartialEq, Eq, Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Filters {
    pub ignore: Vec<Ignore>,
    #[serde(deserialize_with = "deserialize_fancy_regexes")]
    pub regex: Vec<FancyRegex>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Ignore {
    User(String),
    UserInChannel { user: String, channel: String },
    Regex { regex: FancyRegex },
    RegexInChannel { regex: FancyRegex, channel: String },
}

impl<'de> Deserialize<'de> for Ignore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        pub enum Inner {
            User(String),
            UserInChannel { user: String, channel: String },
            Regex { regex: String },
            RegexInChannel { regex: String, channel: String },
        }

        match Inner::deserialize(deserializer)? {
            Inner::User(user) => Ok(Ignore::User(user)),
            Inner::UserInChannel { user, channel } => {
                Ok(Ignore::UserInChannel { user, channel })
            }
            Inner::Regex { regex } => {
                let regex =
                    RegexBuilder::new(&regex).build().map_err(|err| {
                        serde::de::Error::custom(format!(
                            "invalid regex '{regex}': {err}"
                        ))
                    })?;

                Ok(Ignore::Regex {
                    regex: FancyRegex(regex),
                })
            }
            Inner::RegexInChannel { regex, channel } => {
                let regex =
                    RegexBuilder::new(&regex).build().map_err(|err| {
                        serde::de::Error::custom(format!(
                            "invalid regex '{regex}': {err}"
                        ))
                    })?;

                Ok(Ignore::RegexInChannel {
                    regex: FancyRegex(regex),
                    channel,
                })
            }
        }
    }
}

// We want to build the regex on deserialization to present any errors to the
// user then, but we need to be able to define PartialEq and Eq. So, we use this
// Regex wrapper.
#[derive(Debug, Clone)]
pub struct FancyRegex(pub(crate) Regex);

impl PartialEq for FancyRegex {
    fn eq(&self, other: &FancyRegex) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for FancyRegex {}

impl From<FancyRegex> for Regex {
    fn from(fancy_regex: FancyRegex) -> Regex {
        fancy_regex.0
    }
}

pub fn deserialize_fancy_regexes<'de, D>(
    deserializer: D,
) -> Result<Vec<FancyRegex>, D::Error>
where
    D: Deserializer<'de>,
{
    let regex_strings = Vec::<String>::deserialize(deserializer)?;

    regex_strings
        .iter()
        .map(|regex_string| {
            RegexBuilder::new(regex_string)
                .build()
                .map_err(|err| {
                    serde::de::Error::custom(format!(
                        "invalid regex '{regex_string}': {err}"
                    ))
                })
                .map(FancyRegex)
        })
        .collect::<Result<Vec<FancyRegex>, D::Error>>()
}
