use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Deserializer};

use crate::{message::Content, user::NickRef, Message};

#[derive(Debug, Clone, Deserialize)]
pub struct Highlights(Vec<Highlight>);

impl Default for Highlights {
    fn default() -> Self {
        Self(vec![Highlight::Nickname {
            exclude: vec![],
            include: vec![],
        }])
    }
}

impl Highlights {
    pub fn should_highlight_text(
        &self,
        text: &str,
        target: &str,
        sender: NickRef,
        own_nick: NickRef,
    ) -> bool {
        self.0
            .iter()
            .any(|highlight| highlight.from_text(text, target, sender, own_nick))
    }

    pub fn should_highlight_message(
        &self,
        message: &Message,
        sender: NickRef,
        own_nick: NickRef,
    ) -> bool {
        self.0
            .iter()
            .any(|highlight| highlight.from_message(message, sender, own_nick))
    }

    pub fn with_default(&mut self) {
        let default = Self::default();
        self.0.extend(default.0);
    }

    pub fn contains_nickname(&self) -> bool {
        self.0
            .iter()
            .any(|h| matches!(h, Highlight::Nickname { .. }))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", untagged)]
pub enum Highlight {
    Words {
        words: Vec<String>,
        #[serde(default)]
        case_sensitive: bool,
        #[serde(default)]
        exclude: Vec<String>,
        #[serde(default)]
        include: Vec<String>,
    },
    Regex {
        #[serde(deserialize_with = "deserialize_regex")]
        regex: Regex,
        #[serde(default)]
        exclude: Vec<String>,
        #[serde(default)]
        include: Vec<String>,
    },
    Nickname {
        #[serde(default)]
        exclude: Vec<String>,
        #[serde(default)]
        include: Vec<String>,
    },
}

// Custom deserialization function for `Regex`
fn deserialize_regex<'de, D>(deserializer: D) -> Result<Regex, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Regex::new(&s).map_err(|e| serde::de::Error::custom(format!("invalid regex '{}': {}", s, e)))
}

impl Highlight {
    fn include(&self) -> &Vec<String> {
        match self {
            Highlight::Words { include, .. } => include,
            Highlight::Regex { include, .. } => include,
            Highlight::Nickname { include, .. } => include,
        }
    }

    fn exclude(&self) -> &Vec<String> {
        match self {
            Highlight::Words { exclude, .. } => exclude,
            Highlight::Regex { exclude, .. } => exclude,
            Highlight::Nickname { exclude, .. } => exclude,
        }
    }

    fn is_target_included(&self, target: &str) -> bool {
        let include = self.include();
        let exclude = self.exclude();

        let is_channel_filtered = |list: &[String], target: &str| -> bool {
            let wildcards = ["*", "all"];
            list.iter()
                .any(|item| wildcards.contains(&item.as_str()) || item == target)
        };

        let channel_included = is_channel_filtered(include, target);
        let channel_excluded = is_channel_filtered(exclude, target);

        channel_included || !channel_excluded
    }

    fn from_text(&self, text: &str, target: &str, sender: NickRef, own_nick: NickRef) -> bool {
        // Return if sender is yourself.
        if sender == own_nick {
            return false;
        }

        // Target is not included in highlight
        if !self.is_target_included(target) {
            return false;
        }

        self.should_trigger_from_text(text, own_nick)
    }

    fn from_message(&self, message: &Message, sender: NickRef, own_nick: NickRef) -> bool {
        // Return if sender is yourself.
        if sender == own_nick {
            return false;
        }

        // Message has no target
        let target = match &message.target {
            crate::message::Target::Channel { channel, .. } => Some(channel.to_target()),
            crate::message::Target::Query { query, .. } => Some(query.to_target()),
            _ => None,
        };

        let Some(target) = target else {
            return false;
        };

        // Target is not included in highlight
        if !self.is_target_included(target.as_str()) {
            return false;
        }

        match &message.content {
            Content::Plain(text) => self.should_trigger_from_text(text, own_nick),
            Content::Fragments(fragments) => fragments
                .iter()
                .any(|f| self.should_trigger_from_text(f.as_str(), own_nick)),
            Content::Log(_) => false,
        }
    }

    fn should_trigger_from_text(&self, text: &str, own_nick: NickRef) -> bool {
        text.chars()
            .filter(|&c| c != '\u{1}')
            .group_by(|c| c.is_whitespace())
            .into_iter()
            .any(|(is_whitespace, chars)| {
                if !is_whitespace {
                    let text = chars.collect::<String>();

                    match self {
                        Highlight::Words {
                            words,
                            case_sensitive,
                            ..
                        } => words
                            .iter()
                            .any(|word| text_references_word(&text, word, *case_sensitive)),
                        Highlight::Regex { regex, .. } => regex.is_match(&text),
                        Highlight::Nickname { .. } => {
                            text_references_nickname(&text, own_nick).is_some()
                        }
                    }
                } else {
                    false
                }
            })
    }
}

/// Checks if a given `text` contains or matches a user's nickname.
pub fn text_references_nickname(text: &str, own_nick: NickRef) -> Option<bool> {
    let nick = own_nick.as_ref();
    let nick_lower = nick.to_ascii_lowercase();
    let lower = text.to_ascii_lowercase();
    let trimmed = text.trim_matches(|c: char| c.is_ascii_punctuation());
    let lower_trimmed = trimmed.to_ascii_lowercase();

    if nick == text || nick_lower == lower {
        // Contains the user's nickname without trimming.
        Some(false)
    } else if nick == trimmed || nick_lower == lower_trimmed {
        // Contains the user's nickname with trimming.
        Some(true)
    } else {
        // Doesn't contain the user's nickname.
        None
    }
}

/// Checks if a given `text` contains or matches a user's nickname.
// pub fn text_references_nickname_regex(text: &str, own_nick: NickRef) -> bool {
//     let patteren = format!(r"(?i)(?<!\w){}(?!\w)", regex::escape(own_nick.as_ref()));
//     let Ok(re) = Regex::new(&patteren) else {
//         return false;
//     };

//     re.is_match(text).unwrap_or_default()
// }

/// Checks if a given `text` contains or matches a word.
pub fn text_references_word(text: &str, word: &str, case_sensitive: bool) -> bool {
    let text = text
        .trim_matches(|c: char| c.is_ascii_punctuation())
        .to_string();

    if case_sensitive {
        word == &text
    } else {
        word.eq_ignore_ascii_case(&text)
    }
}
