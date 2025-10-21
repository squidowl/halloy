use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(default)]
pub struct PlatformSpecific {
    pub linux: Linux,
    pub macos: MacOS,
    pub windows: Windows,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Windows {
    pub decorations: bool,
}

impl Default for Windows {
    fn default() -> Self {
        Self { decorations: true }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Linux {
    pub decorations: bool,
}

impl Default for Linux {
    fn default() -> Self {
        Self { decorations: true }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct MacOS {
    pub decorations: bool,
    pub content_padding: TitlebarPadding,
    pub sidebar_padding: TitlebarPadding,
}

impl Default for MacOS {
    fn default() -> Self {
        Self {
            decorations: true,
            content_padding: TitlebarPadding::default(),
            sidebar_padding: TitlebarPadding::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TitlebarPadding {
    #[default]
    EmbeddedContent,
    PaddedContent,
}
