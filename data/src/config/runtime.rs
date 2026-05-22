use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Runtime {
    pub backend: Backend,
    pub vsync: bool,
    pub antialiasing: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            backend: Backend::default(),
            vsync: true,
            antialiasing: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Backend {
    #[default]
    Best,
    Hardware,
    Software,
}
