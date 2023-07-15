use data::config;

#[allow(dead_code)]
const APP_ID: &str = "irc.squidowl.org";

#[cfg(target_os = "macos")]
pub fn prepare() {
    match notify_rust::set_application(APP_ID) {
        Ok(_) => {}
        Err(error) => {
            log::error!("{}", error.to_string());
        }
    }
}

#[cfg(not(any(target_os = "macos")))]
pub fn prepare() {}

#[derive(Default)]
pub struct Notification {
    body: Option<String>,
    title: Option<String>,
    sound: Option<String>,
}

impl Notification {
    pub fn new(config: &config::notification::Config) -> Notification {
        Notification {
            sound: config.sound.clone(),
            ..Default::default()
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn show(self) {
        let mut notification = notify_rust::Notification::new();

        if let Some(body) = self.body {
            notification.body(&body);
        }

        if let Some(title) = self.title {
            notification.summary(&title);
        }

        if let Some(sound) = self.sound {
            notification.sound_name(&sound);
        }

        #[cfg(windows)]
        {
            notification.app_id(APP_ID);
        }

        let _ = notification.show();
    }
}
