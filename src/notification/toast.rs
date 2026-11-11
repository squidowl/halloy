use data::config::actions::NotificationAction;
#[cfg(target_os = "linux")]
use image::EncodableLayout;
use notify_rust::Notification;
#[cfg(target_os = "linux")]
use notify_rust::NotificationResponse;

#[cfg(target_os = "macos")]
pub fn prepare() {
    match notify_rust::set_application(data::environment::APPLICATION_ID) {
        Ok(()) => {}
        Err(error) => {
            log::error!("error preparing notification backend: {error}");
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn prepare() {}

pub struct Toast(Notification);

impl Toast {
    pub fn new(
        title: &str,
        subtitle: Option<&str>,
        body: &str,
        has_buffer_context: bool,
        notification_action: NotificationAction,
    ) -> Self {
        let mut notification = notify_rust::Notification::new();

        notification.body(body);

        #[cfg(target_os = "macos")]
        {
            notification.summary(title);
            if let Some(subtitle) = subtitle {
                notification.subtitle(subtitle);
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            if let Some(subtitle) = subtitle {
                notification.summary(&format!("{title} ({subtitle})"));
            } else {
                notification.summary(title);
            }
            notification.appname("Halloy");
            notification.icon(data::environment::APPLICATION_ID);
        }
        #[cfg(target_os = "linux")]
        {
            // For GNOME 46+ setting the icon is not sufficient to show the icon
            // in the body area of the notification; setting image_data or
            // image_path is needed.
            if let Some(logo) = image::load_from_memory_with_format(
                include_bytes!("../../assets/logo.png"),
                image::ImageFormat::Png,
            )
            .ok()
            .and_then(|image| {
                image.as_rgba8().and_then(|image| {
                    notify_rust::Image::from_rgba(
                        image.width().try_into().unwrap_or_default(),
                        image.height().try_into().unwrap_or_default(),
                        image.as_bytes().to_vec(),
                    )
                    .ok()
                })
            }) {
                notification.image_data(logo);
            }
        }
        #[cfg(target_os = "windows")]
        {
            notification.app_id(data::environment::APPLICATION_ID);
        }

        match notification_action {
            NotificationAction::ActivateApplication => {
                notification.action("default", "Open Halloy");
                if has_buffer_context {
                    notification.action("open_buffer", "Open Buffer");
                }
            }
            NotificationAction::OpenBuffer => {
                if has_buffer_context {
                    notification.action("default", "Open Buffer");
                }
            }
        }

        Self(notification.finalize())
    }

    #[cfg(target_os = "linux")]
    pub async fn show_and_wait_for_response(
        self,
        default_action: NotificationAction,
    ) -> Option<Action> {
        let mut action = None;

        // When image_data is set, Notification::show/Notification::show_async
        // will attempt to start a tokio runtime and panic.  This is a
        // workaround for that behavior.
        if let Ok(handle) = tokio::task::spawn_blocking(move || {
            futures::executor::block_on(async { self.0.show_async().await })
        })
        .await
        .ok()?
        {
            handle
                .wait_for_action_async(|response: &NotificationResponse| {
                    Toast::handle_response(
                        response,
                        default_action,
                        &mut action,
                    );
                })
                .await;
        }

        action
    }

    #[cfg(not(target_os = "linux"))]
    pub fn show(self) {
        if let Err(error) = self.0.show() {
            log::error!("error showing toast: {error}");
        }
    }

    #[cfg(target_os = "linux")]
    fn handle_response(
        response: &NotificationResponse,
        default_action: NotificationAction,
        action: &mut Option<Action>,
    ) {
        match response {
            NotificationResponse::Default => match default_action {
                NotificationAction::ActivateApplication => {
                    *action = Some(Action::ActivateApplication);
                }
                NotificationAction::OpenBuffer => {
                    *action = Some(Action::OpenBuffer);
                }
            },
            NotificationResponse::Action(response_action)
                if response_action == "open_buffer" =>
            {
                *action = Some(Action::OpenBuffer);
            }
            NotificationResponse::Action(_)
            | NotificationResponse::Reply(_)
            | NotificationResponse::Closed(_) => {
                *action = Some(Action::Dismiss);
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    ActivateApplication,
    OpenBuffer,
    Dismiss,
}
