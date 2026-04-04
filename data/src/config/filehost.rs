use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Filehost {
    /// Master switch to disable all file upload functionality. Defaults to `true`.
    pub enabled: bool,
    /// Show the upload button in the input bar. Defaults to `true`.
    pub button: bool,
    /// Handle uploaded files from the clipboard via paste. Defaults to `true`.
    pub paste: bool,
    /// Handle drag-and-drop. Defaults to `true`.
    pub file_drop: bool,
}

impl Default for Filehost {
    fn default() -> Self {
        Self {
            enabled: true,
            button: true,
            paste: true,
            file_drop: false,
        }
    }
}

impl Filehost {
    pub fn button(&self) -> bool {
        self.enabled && self.button
    }

    pub fn paste(&self) -> bool {
        self.enabled && self.paste
    }

    pub fn file_drop(&self) -> bool {
        self.enabled && self.file_drop
    }
}
