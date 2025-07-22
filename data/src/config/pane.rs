use serde::Deserialize;

use crate::config::Scrollbar;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Pane {
    /// Default axis used when splitting a pane.
    pub split_axis: SplitAxis,
    pub scrollbar: Scrollbar,
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SplitAxis {
    #[default]
    Horizontal,
    Vertical,
}
