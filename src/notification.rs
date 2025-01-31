use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};

use data::{
    audio::Sound,
    client::Notification,
    config::{self, notification},
};

use crate::audio;

pub use self::toast::prepare;

mod toast;

pub struct Notifications {
    recent_notifications: HashMap<Notification, DateTime<Utc>>,
}

impl Notifications {
    pub fn new() -> Self {
        Self {
            recent_notifications: HashMap::new(),
        }
    }

    pub fn notify(
        &mut self,
        config: &config::Notifications<Sound>,
        notification: &Notification,
        server: Option<impl ToString>,
    ) {
        match notification {
            Notification::Connected => {
                if let Some(server) = server {
                    self.execute(
                        &config.connected,
                        notification,
                        "Connected",
                        server.to_string(),
                    );
                }
            }
            Notification::Disconnected => {
                if let Some(server) = server {
                    self.execute(
                        &config.disconnected,
                        notification,
                        "Disconnected",
                        server.to_string(),
                    );
                }
            }
            Notification::Reconnected => {
                if let Some(server) = server {
                    self.execute(
                        &config.reconnected,
                        notification,
                        "Reconnected",
                        server.to_string(),
                    );
                }
            }
            Notification::MonitoredOnline(targets) => {
                if let Some(server) = server {
                    targets.iter().for_each(|target| {
                        self.execute(
                            &config.monitored_online,
                            notification,
                            &format!("{} is online", target.nickname()),
                            server.to_string(),
                        );
                    });
                }
            }
            Notification::MonitoredOffline(targets) => {
                if let Some(server) = server {
                    targets.iter().for_each(|target| {
                        self.execute(
                            &config.monitored_offline,
                            notification,
                            &format!("{} is offline", target),
                            server.to_string(),
                        );
                    });
                }
            }
            Notification::FileTransferRequest(nick) => {
                if let Some(server) = server {
                    if config
                        .file_transfer_request
                        .should_notify(vec![nick.to_string()])
                    {
                        self.execute(
                            &config.file_transfer_request,
                            notification,
                            &format!("File transfer from {}", nick),
                            server.to_string(),
                        );
                    }
                }
            }
            Notification::DirectMessage(user) => {
                if config
                    .direct_message
                    .should_notify(vec![user.nickname().to_string()])
                {
                    self.execute(
                        &config.direct_message,
                        notification,
                        "Direct message",
                        format!("{} sent you a direct message", user.nickname()),
                    );
                }
            }
            Notification::Highlight {
                enabled,
                user,
                target,
            } => {
                if config
                    .highlight
                    .should_notify(vec![target.to_string(), user.nickname().to_string()])
                    && *enabled
                {
                    self.execute(
                        &config.highlight,
                        notification,
                        "Highlight",
                        format!("{} highlighted you in {}", user.nickname(), target),
                    );
                }
            }
        }
    }

    fn execute(
        &mut self,
        config: &notification::Loaded,
        notification: &Notification,
        title: &str,
        body: impl ToString,
    ) {
        let last_notification = self.recent_notifications.get(notification).cloned();

        if last_notification.is_some()
            && last_notification.unwrap()
                > Utc::now() - Duration::from_millis(config.delay.unwrap_or(500))
        {
            return;
        }

        if config.show_toast {
            toast::show(title, body);
        }

        if let Some(sound) = &config.sound {
            audio::play(sound.clone());
        }

        self.recent_notifications
            .insert(notification.clone(), Utc::now());
    }
}
