use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use data::audio::Sound;
use data::config::{self, notification};
use data::target::join_targets;
use data::user::Nick;
use data::{Config, Notification, Server, User};

pub use self::toast::prepare;
use crate::audio;

mod toast;

#[derive(PartialEq, Eq, Hash, Clone)]
enum NotificationDelayKey {
    Connected,
    Disconnected,
    Reconnected,
    DirectMessage(Box<str>),
    Highlight(Box<str>),
    FileTransferRequest(Box<str>),
    MonitoredOnline,
    MonitoredOffline,
    Channel(Box<str>),
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
            Notification::Highlight { channel, .. } => {
                NotificationDelayKey::Highlight(
                    channel.as_normalized_str().into(),
                )
            }
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
            Notification::Channel { channel, .. } => {
                NotificationDelayKey::Channel(
                    channel.as_normalized_str().into(),
                )
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
        let sounds =
            config.notifications.load_sounds(
                config.highlights.matches.iter().filter_map(
                    |highlight_match| highlight_match.sound.as_deref(),
                ),
            );

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
                    None,
                );
            }
            Notification::Disconnected => {
                self.execute(
                    &config.disconnected,
                    notification,
                    "Disconnected",
                    &server.to_string(),
                    None,
                );
            }
            Notification::Reconnected => {
                self.execute(
                    &config.reconnected,
                    notification,
                    "Reconnected",
                    &server.to_string(),
                    None,
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
                    &join_targets(targets.iter().map(User::as_str).collect()),
                    None,
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
                    &join_targets(targets.iter().map(Nick::as_str).collect()),
                    None,
                );
            }
            Notification::FileTransferRequest {
                nick,
                casemapping,
                filename,
            } => {
                if config.file_transfer_request.should_notify(
                    &User::from(nick.clone()),
                    None,
                    server,
                    *casemapping,
                ) {
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
                        None,
                    );
                }
            }
            Notification::DirectMessage {
                user,
                casemapping,
                message,
            } => {
                if config.direct_message.should_notify(
                    user,
                    None,
                    server,
                    *casemapping,
                ) {
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
                        None,
                    );
                }
            }
            Notification::Highlight {
                user,
                channel,
                casemapping,
                message,
                description,
                sound,
            } => {
                if config.highlight.should_notify(
                    user,
                    Some(channel),
                    server,
                    *casemapping,
                ) {
                    // Description is expected to be expanded by the calling
                    // routine when show_content is true
                    if config.highlight.show_content {
                        self.execute(
                            &config.highlight,
                            notification,
                            &format!(
                                "{} {description} in {channel} on {server}",
                                user.nickname()
                            ),
                            message,
                            sound.as_deref(),
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
                            sound.as_deref(),
                        );
                    }
                }
            }
            Notification::Channel {
                user,
                channel,
                casemapping,
                message,
            } => {
                if let Some(notification_config) =
                    config.channels.get(channel.as_str())
                    && notification_config.should_notify(
                        user,
                        None,
                        server,
                        *casemapping,
                    )
                {
                    if notification_config.show_content {
                        self.execute(
                            notification_config,
                            notification,
                            &format!(
                                "{} sent a message in {channel} on {server}",
                                user.nickname()
                            ),
                            message,
                            None,
                        );
                    } else {
                        self.execute(
                            notification_config,
                            notification,
                            &format!(
                                "{} sent a message in {channel}",
                                user.nickname()
                            ),
                            &server.name,
                            None,
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
        sound_name: Option<&str>,
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

        if let Some(sound) = sound_name
            .or(config.sound.as_deref())
            .and_then(|sound_name| self.sounds.get(sound_name))
        {
            audio::play(sound.clone());
        }
    }
}
