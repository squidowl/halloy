use std::fmt;
use std::hash::Hash;

use ecow::EcoString;
use serde::{Deserialize, Serialize};

use crate::casemapping::CaseMap;

/// A string type for small, rarely modified but often copied strings.
pub type IrcString = EcoString;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseMapped {
    raw: IrcString,
    normalized: IrcString,
}

impl CaseMapped {
    pub fn as_normalized_str(&self) -> &str {
        self.normalized.as_ref()
    }

    pub fn as_str(&self) -> &str {
        self.raw.as_ref()
    }

    pub fn from_string(s: impl Into<IrcString>, casemapping: CaseMap) -> Self {
        let s = s.into();
        Self {
            normalized: casemapping.normalize(s.as_str()).into(),
            raw: s,
        }
    }

    pub fn renormalize(&mut self, casemapping: CaseMap) {
        // TODO(pounce, perf) rc++ if this is the same as before?
        self.normalized = casemapping.normalize(&self.raw).into();
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
