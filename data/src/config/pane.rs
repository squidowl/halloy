use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Pane {
    /// Default axis used when splitting a pane.
    #[serde(default)]
    pub split_axis: SplitAxis,
    /// Scrollbar configuration
    #[serde(default)]
    pub scrollbar: Scrollbar,
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SplitAxis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
pub struct Scrollbar {
    /// Width of the scrollbar.
    #[serde(default = "default_scrollbar_width")]
    pub width: u32,
    /// Width of the scrollbar scroller.
    #[serde(default = "default_scrollbar_scroller_width")]
    pub scroller_width: u32,
}

fn default_scrollbar_width() -> u32 {
    5
}

fn default_scrollbar_scroller_width() -> u32 {
    5
}
