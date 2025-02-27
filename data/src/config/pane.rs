use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Pane {
    /// Default axis used when splitting a pane.
    #[serde(default)]
    pub split_axis: SplitAxis,
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SplitAxis {
    #[default]
    Horizontal,
    Vertical,
}
