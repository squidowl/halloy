use data::Config;
use data::config::{platform_specific, sidebar};

/// Returns the content padding based on platform and configuration..
pub fn content_padding(config: &Config) -> u32 {
    // On macOS the content is either embedded into titlebar or padded below it.
    if cfg!(target_os = "macos") {
        let padding = match config.platform_specific.macos.content_padding {
            platform_specific::TitlebarPadding::EmbeddedContent => 0,
            platform_specific::TitlebarPadding::PaddedContent => 20,
        };

        match config.sidebar.position {
            sidebar::Position::Left => padding,
            sidebar::Position::Right => padding,
            sidebar::Position::Top => 0,
            sidebar::Position::Bottom => padding,
        }
    } else {
        0
    }
}

/// Returns the sidebar padding based on platform and configuration..
pub fn sidebar_padding(config: &Config) -> u32 {
    if cfg!(target_os = "macos") {
        let padding = match config.platform_specific.macos.sidebar_padding {
            platform_specific::TitlebarPadding::EmbeddedContent => 0,
            platform_specific::TitlebarPadding::PaddedContent => 20,
        };

        match config.sidebar.position {
            sidebar::Position::Left => padding,
            sidebar::Position::Right => padding,
            sidebar::Position::Top => padding,
            sidebar::Position::Bottom => 0,
        }
    } else {
        0
    }
}
