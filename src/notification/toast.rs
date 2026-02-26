#[cfg(target_os = "macos")]
pub fn prepare() {
    match notify_rust::set_application(data::environment::APPLICATION_ID) {
        Ok(()) => {}
        Err(error) => {
            log::error!("{error}");
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn prepare() {}

pub fn show(title: &str, subtitle: Option<&str>, body: &str) {
    let mut notification = notify_rust::Notification::new();

    notification.body(body);

    #[cfg(not(target_os = "linux"))]
    {
        notification.summary(title);
        if let Some(subtitle) = subtitle {
            notification.subtitle(subtitle);
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(subtitle) = subtitle {
			notification.summary(&format!("{title} ({subtitle})"));
		} else {
			notification.summary(title);
		}
        notification.appname("Halloy");
        notification.icon(data::environment::APPLICATION_ID);
    }
    #[cfg(target_os = "windows")]
    {
        notification.app_id(data::environment::APPLICATION_ID);
    }

    let _ = notification.show();
}
