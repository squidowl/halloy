use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Tooltips {
    #[serde(default = "default_tooltip")]
    pub button: bool,
    #[serde(default = "default_tooltip")]
    pub user: bool,
}

impl Default for Tooltips {
    fn default() -> Self {
        Self {
            button: default_tooltip(),
            user: default_tooltip(),
        }
    }
}

fn default_tooltip() -> bool {
    true
}
