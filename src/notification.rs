use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use data::audio::Sound;
use data::config::{self, notification};
use data::{Notification, Server};

pub use self::toast::prepare;
use crate::audio;

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
        server: &Server,
    ) {
        match notification {
            Notification::Connected => {
                self.execute(
                    &config.connected,
                    notification,
                    "Connected",
                    server,
                );
            }
            Notification::Disconnected => {
                self.execute(
                    &config.disconnected,
                    notification,
                    "Disconnected",
                    server,
                );
            }
            Notification::Reconnected => {
                self.execute(
                    &config.reconnected,
                    notification,
                    "Reconnected",
                    server,
                );
            }
            Notification::MonitoredOnline(targets) => {
                targets.iter().for_each(|target| {
                    self.execute(
                        &config.monitored_online,
                        notification,
                        &format!("{} is online", target.nickname()),
                        server,
                    );
                });
            }
            Notification::MonitoredOffline(targets) => {
                targets.iter().for_each(|target| {
                    self.execute(
                        &config.monitored_offline,
                        notification,
                        &format!("{target} is offline"),
                        server,
                    );
                });
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
                            filename.as_ref(),
                        )
                    } else {
                        (&format!("File transfer from {nick}"), server.as_ref())
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
                            message.as_ref(),
                        )
                    } else {
                        (
                            &format!(
                                "{} sent you a direct message",
                                user.nickname()
                            ),
                            server.as_ref(),
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
                    let (title, body) = if config.highlight.show_content {
                        (
                            &format!(
                                "{} {description} in {channel} on {server}",
                                user.nickname()
                            ),
                            message.as_ref(),
                        )
                    } else {
                        (
                            &format!(
                                "{} {description} in {channel}",
                                user.nickname()
                            ),
                            server.as_ref(),
                        )
                    };

                    self.execute(&config.highlight, notification, title, body);
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
        let last_notification =
            self.recent_notifications.get(notification).copied();

        if last_notification.is_some()
            && last_notification.unwrap()
                > Utc::now()
                    - Duration::from_millis(config.delay.unwrap_or(500))
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
