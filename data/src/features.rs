use std::sync::LazyLock;

pub static DEFAULT: LazyLock<Features> = LazyLock::new(Features::default);

// These are only features which are not advertised vis IRCv3 CAP or ISUPPORT.
#[derive(Debug, Default)]
pub struct Features {
    pub detach: bool,
    pub mass_message: bool,
    pub list_mode_with_equal: bool,
    pub version_request: VersionRequest,
}

#[derive(Debug)]
pub enum VersionRequest {
    Need(bool),
    Sent,
}

impl Default for VersionRequest {
    fn default() -> Self {
        Self::Need(false)
    }
}

impl Features {
    pub fn enable_supported(&mut self, server_version: &str) {
        if server_version.starts_with("soju") {
            self.detach = true;
            self.version_request = VersionRequest::Need(true);
        } else if server_version.starts_with("ergo") {
            self.mass_message = true;
        } else if server_version.starts_with("solanum") {
            self.mass_message = true;
            self.list_mode_with_equal = true;
        }
    }
}
