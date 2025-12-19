use serde::Deserialize;

pub use self::notice::Notice;

pub mod notice;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Event {
    pub notice: Notice,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            notice: Notice::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Route {
    ActiveBuffer,
    Server,
}