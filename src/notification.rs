#[cfg(target_os = "macos")]
pub fn prepare() {
    match notify_rust::set_application(data::environment::APPLICATION_ID) {
        Ok(_) => {}
        Err(error) => {
            log::error!("{}", error.to_string());
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn prepare() {}

pub fn show(title: &str, body: impl ToString, sound: Option<&str>) {
    let mut notification = notify_rust::Notification::new();

    notification.summary(title);
    notification.body(&body.to_string());

    if let Some(sound) = sound {
        notification.sound_name(sound);
    }

    #[cfg(target_os = "linux")]
    {
        notification.appname("Halloy");
        notification.icon(data::environment::APPLICATION_ID);
    }
    #[cfg(target_os = "windows")]
    {
        notification.app_id(data::environment::APPLICATION_ID);
    }

    let _ = notification.show();
}
