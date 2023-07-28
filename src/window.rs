pub use iced::window::{close, Settings};

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn settings() -> Settings {
    Default::default()
}

#[cfg(target_os = "linux")]
pub fn settings() -> Settings {
    use data::environment;
    use iced::window;

    Settings {
        platform_specific: window::PlatformSpecific {
            application_id: environment::APPLICATION_ID.to_string(),
        },
        ..Default::default()
    }
}

#[cfg(target_os = "macos")]
pub fn settings() -> Settings {
    use iced::window;

    Settings {
        platform_specific: window::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        ..Default::default()
    }
}

#[cfg(target_os = "windows")]
pub fn settings() -> Settings {
    use iced::window;
    use image::EncodableLayout;

    let img = image::load_from_memory_with_format(
        include_bytes!("../assets/logo.png"),
        image::ImageFormat::Png,
    );
    match img {
        Ok(img) => match img.as_rgba8() {
            Some(icon) => Settings {
                icon: window::icon::from_rgba(
                    icon.as_bytes().to_vec(),
                    icon.width(),
                    icon.height(),
                )
                .ok(),
                ..Default::default()
            },
            None => Default::default(),
        },
        Err(_) => Settings {
            ..Default::default()
        },
    }
}
