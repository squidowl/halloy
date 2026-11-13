use std::fmt;
use std::hash::Hash;

use ecow::EcoString;
use serde::{Deserialize, Serialize};

use crate::casemapping::CaseMap;

/// A string type for small, rarely modified but often copied strings.
pub type IrcString = EcoString;

#[derive(Debug, Clone, Serialize)]
pub struct CaseMapped {
    raw: IrcString,
    normalized: IrcString,
}

impl CaseMapped {
    fn new(raw: IrcString, normalized: IrcString) -> Self {
        Self {
            normalized: if normalized == raw {
                raw.clone()
            } else {
                normalized
            },
            raw,
        }
    }
}

// We need to use CaseMapped::new to ensure string deduplication.
impl<'de> Deserialize<'de> for CaseMapped {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "CaseMapped")]
        struct Stored {
            raw: IrcString,
            normalized: IrcString,
        }

        let stored = Stored::deserialize(deserializer)?;
        Ok(Self::new(stored.raw, stored.normalized))
    }
}

impl CaseMapped {
    pub fn as_normalized_str(&self) -> &str {
        self.normalized.as_ref()
    }

    pub fn as_str(&self) -> &str {
        self.raw.as_ref()
    }

    pub fn from_string(s: impl Into<IrcString>, casemapping: CaseMap) -> Self {
        let raw = s.into();
        let normalized = casemapping.normalize(&raw).into();
        Self::new(raw, normalized)
    }

    pub fn renormalize(&mut self, casemapping: CaseMap) {
        let normalized: IrcString = casemapping.normalize(self.as_str()).into();
        if normalized.as_str() == self.normalized {
            return;
        }
        self.normalized = if normalized == self.raw {
            self.raw.clone()
        } else {
            normalized
        };
    }
}

impl PartialEq for CaseMapped {
    fn eq(&self, other: &Self) -> bool {
        self.normalized.eq(&other.normalized)
    }
}

impl Eq for CaseMapped {}

impl PartialOrd for CaseMapped {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CaseMapped {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.normalized.cmp(&other.normalized)
    }
}

impl Hash for CaseMapped {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.normalized.hash(state);
    }
}

impl fmt::Display for CaseMapped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.fmt(f)
    }
}
