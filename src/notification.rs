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
    notified_times: HashMap<Notification, DateTime<Utc>>,
}

impl Notifications {
    pub fn new() -> Self {
        Self {
            notified_times: HashMap::new(),
        }
    }

    pub fn notify(
        &mut self,
        config: &config::Notifications<Sound>,
        notification: &Notification,
        server: Option<impl ToString>,
    ) {
        if let Some(server) = server {
            match notification {
                Notification::Connected => {
                    self._execute(
                        &config.connected,
                        notification,
                        "Connected",
                        server.to_string(),
                    );
                }
                Notification::Disconnected => {
                    self._execute(
                        &config.disconnected,
                        notification,
                        "Disconnected",
                        server.to_string(),
                    );
                }
                Notification::Reconnected => {
                    self._execute(
                        &config.reconnected,
                        notification,
                        "Reconnected",
                        server.to_string(),
                    );
                }
                Notification::MonitoredOnline(targets) => {
                    targets.iter().for_each(|target| {
                        self._execute(
                            &config.monitored_online,
                            notification,
                            &format!("{} is online", target.nickname()),
                            server.to_string(),
                        );
                    });
                }
                Notification::MonitoredOffline(targets) => {
                    targets.iter().for_each(|target| {
                        self._execute(
                            &config.monitored_offline,
                            notification,
                            &format!("{} is offline", target),
                            server.to_string(),
                        );
                    });
                }
                Notification::FileTransferRequest(nick) => {
                    self._execute(
                        &config.file_transfer_request,
                        notification,
                        &format!("File transfer from {}", nick),
                        server.to_string(),
                    );
                }
                _ => {}
            }
        }

        match notification {
            Notification::DirectMessage(user) => {
                self._execute(
                    &config.direct_message,
                    notification,
                    "Direct message",
                    format!("{} sent you a direct message", user.nickname()),
                );
            }
            Notification::Highlight {
                enabled,
                user,
                channel,
            } => {
                if *enabled {
                    self._execute(
                        &config.highlight,
                        notification,
                        "Highlight",
                        format!("{} highlighted you in {}", user.nickname(), channel),
                    );
                }
            }
            _ => {}
        }
    }

    fn _execute(
        &mut self,
        config: &notification::Loaded,
        notification: &Notification,
        title: &str,
        body: impl ToString,
    ) {
        let last_notification = self.notified_times.get(notification).cloned();

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

        self.notified_times.insert(notification.clone(), Utc::now());
    }
}
