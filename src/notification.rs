use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use data::audio::Sound;
use data::config::{self, notification};
use data::{Config, Notification, Server, User};
use itertools::Itertools;

pub use self::toast::prepare;
use crate::audio;

mod toast;

#[derive(PartialEq, Eq, Hash, Clone)]
enum NotificationDelayKey {
    Connected,
    Disconnected,
    Reconnected,
    DirectMessage(Box<str>),
    Highlight,
    FileTransferRequest(Box<str>),
    MonitoredOnline,
    MonitoredOffline,
}

impl From<&Notification> for NotificationDelayKey {
    fn from(notification: &Notification) -> NotificationDelayKey {
        match notification {
            Notification::Connected => NotificationDelayKey::Connected,
            Notification::Disconnected => NotificationDelayKey::Disconnected,
            Notification::Reconnected => NotificationDelayKey::Reconnected,
            Notification::DirectMessage { user, .. } => {
                NotificationDelayKey::DirectMessage(
                    user.nickname().as_normalized_str().into(),
                )
            }
            Notification::Highlight { .. } => NotificationDelayKey::Highlight,
            Notification::FileTransferRequest { nick, .. } => {
                NotificationDelayKey::FileTransferRequest(
                    nick.as_normalized_str().into(),
                )
            }
            Notification::MonitoredOnline(..) => {
                NotificationDelayKey::MonitoredOnline
            }
            Notification::MonitoredOffline(..) => {
                NotificationDelayKey::MonitoredOffline
            }
        }
    }
}

pub struct Notifications {
    recent_notifications: HashMap<NotificationDelayKey, DateTime<Utc>>,
    sounds: HashMap<String, Sound>,
}

impl Notifications {
    pub fn new(config: &Config) -> Self {
        // Load sounds from different sources.
        let sounds = config.notifications.load_sounds();

        Self {
            recent_notifications: HashMap::new(),
            sounds,
        }
    }

    pub fn notify(
        &mut self,
        config: &config::Notifications,
        notification: &Notification,
        server: &Server,
    ) {
        match notification {
            Notification::Connected => {
                self.execute(
                    &config.connected,
                    notification,
                    "Connected",
                    &server.to_string(),
                );
            }
            Notification::Disconnected => {
                self.execute(
                    &config.disconnected,
                    notification,
                    "Disconnected",
                    &server.to_string(),
                );
            }
            Notification::Reconnected => {
                self.execute(
                    &config.reconnected,
                    notification,
                    "Reconnected",
                    &server.to_string(),
                );
            }
            Notification::MonitoredOnline(targets) => {
                self.execute(
                    &config.monitored_online,
                    notification,
                    if targets.len() == 1 {
                        "Monitored user is online"
                    } else {
                        "Monitored users are online"
                    },
                    &targets.iter().map(User::nickname).join(", "),
                );
            }
            Notification::MonitoredOffline(targets) => {
                self.execute(
                    &config.monitored_online,
                    notification,
                    if targets.len() == 1 {
                        "Monitored user is offline"
                    } else {
                        "Monitored users are offline"
                    },
                    &targets.iter().join(", "),
                );
            }
            Notification::FileTransferRequest { nick, filename } => {
                if config
                    .file_transfer_request
                    .should_notify(vec![nick.to_string()])
                {
                    let (title, body) = if config
                        .file_transfer_request
                        .show_content
                    {
                        (
                            &format!("File transfer from {nick} on {server}"),
                            filename,
                        )
                    } else {
                        (
                            &format!("File transfer from {nick}"),
                            &format!("{server}"),
                        )
                    };

                    self.execute(
                        &config.file_transfer_request,
                        notification,
                        title,
                        body,
                    );
                }
            }
            Notification::DirectMessage { user, message } => {
                if config
                    .direct_message
                    .should_notify(vec![user.nickname().to_string()])
                {
                    let (title, body) = if config.direct_message.show_content {
                        (
                            &format!(
                                "{} sent you a direct message on {server}",
                                user.nickname()
                            ),
                            message,
                        )
                    } else {
                        (
                            &format!(
                                "{} sent you a direct message",
                                user.nickname()
                            ),
                            &format!("{server}"),
                        )
                    };

                    self.execute(
                        &config.direct_message,
                        notification,
                        title,
                        body,
                    );
                }
            }
            Notification::Highlight {
                user,
                channel,
                message,
                description,
            } => {
                if config.highlight.should_notify(vec![
                    channel.to_string(),
                    user.nickname().to_string(),
                ]) {
                    if config.highlight.show_content {
                        self.execute(
                            &config.highlight,
                            notification,
                            &format!(
                                "{} {description} in {channel} on {server}",
                                user.nickname()
                            ),
                            message,
                        );
                    } else {
                        self.execute(
                            &config.highlight,
                            notification,
                            &format!(
                                "{} {description} in {channel}",
                                user.nickname()
                            ),
                            &server.name,
                        );
                    }
                }
            }
        }
    }

    fn execute(
        &mut self,
        config: &notification::Notification,
        notification: &Notification,
        title: &str,
        body: &str,
    ) {
        let now = Utc::now();
        let delay_key = notification.into();

        if self.recent_notifications.get(&delay_key).is_some_and(|last_notification| {
            now - last_notification
                < TimeDelta::milliseconds(config.delay.unwrap_or(500) as i64)
        }) {
            return;
        }

        self.recent_notifications.insert(delay_key, now);

        if config.show_toast {
            toast::show(title, body);
        }

        if let Some(sound_name) = &config.sound
            && let Some(sound) = self.sounds.get(sound_name)
        {
            audio::play(sound.clone());
        }
    }
}
