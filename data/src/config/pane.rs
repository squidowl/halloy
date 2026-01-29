use serde::Deserialize;

use crate::config::Scrollbar;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Pane {
    /// Default axis used when splitting a pane.
    pub split_axis: SplitAxis,
    pub scrollbar: Scrollbar,
    pub restore_on_launch: bool,
    pub gap: Gap,
}

impl Default for Pane {
    fn default() -> Self {
        Self {
            split_axis: SplitAxis::default(),
            scrollbar: Scrollbar::default(),
            restore_on_launch: true,
            gap: Gap::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(default)]
pub struct Gap {
    pub inner: u32,
    pub outer: u16,
}

impl Default for Gap {
    fn default() -> Self {
        Self {
            inner: 4,
            outer: 8,
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
