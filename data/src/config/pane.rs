use serde::Deserialize;

use crate::config::Scrollbar;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Pane {
    /// Default axis used when splitting a pane.
    pub split_axis: SplitAxis,
    pub scrollbar: Scrollbar,
    pub restore_on_launch: bool,
}

impl Default for Pane {
    fn default() -> Self {
        Self {
            split_axis: SplitAxis::default(),
            scrollbar: Scrollbar::default(),
            restore_on_launch: true,
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SplitAxis {
    Horizontal,
    Vertical,
    #[default]
    Shorter,
    LargestShorter,
}
