use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use data::audio::Sound;
use data::config::{self, notification};
use data::target::join_targets;
use data::user::Nick;
use data::{Config, Notification, Server, User};
use iced::Task;

pub use self::toast::prepare;
use crate::{audio, window};

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

    pub fn notify<Message: 'static + Send>(
        &mut self,
        config: &config::Notifications,
        notification: &Notification,
        server: &Server,
        window_id: window::Id,
    ) -> Option<Task<Message>> {
        let request_attention = match notification {
            Notification::Connected => {
                self.execute(
                    &config.connected,
                    notification,
                    "Connected",
                    None,
                    &server.to_string(),
                    None,
                );

                config.connected.request_attention
            }
            Notification::Disconnected => {
                self.execute(
                    &config.disconnected,
                    notification,
                    "Disconnected",
                    None,
                    &server.to_string(),
                    None,
                );

                config.disconnected.request_attention
            }
            Notification::Reconnected => {
                self.execute(
                    &config.reconnected,
                    notification,
                    "Reconnected",
                    None,
                    &server.to_string(),
                    None,
                );

                config.reconnected.request_attention
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
                    None,
                    &join_targets(targets.iter().map(User::as_str).collect()),
                    None,
                );

                config.monitored_online.request_attention
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
                    None,
                    &join_targets(targets.iter().map(Nick::as_str).collect()),
                    None,
                );

                config.monitored_online.request_attention
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
                    let (title, subtitle, body): (
                        String,
                        Option<String>,
                        String,
                    ) = if config.file_transfer_request.show_content {
                        (
                            nick.as_str().to_owned(),
                            Some(format!("{server}")),
                            format!("Sent you a file: {filename}"),
                        )
                    } else {
                        (
                            format!("File transfer from {nick}"),
                            None,
                            format!("Sent you a file on {server}"),
                        )
                    };

                    self.execute(
                        &config.file_transfer_request,
                        notification,
                        title.as_str(),
                        subtitle.as_deref(),
                        body.as_str(),
                        None,
                    );

                    config.file_transfer_request.request_attention
                } else {
                    false
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
                    let (title, subtitle, body): (
                        String,
                        Option<String>,
                        String,
                    ) = if config.direct_message.show_content {
                        (
                            user.nickname().as_str().to_owned(),
                            Some(format!("{server}")),
                            message.to_owned(),
                        )
                    } else {
                        (
                            user.nickname().as_str().to_owned(),
                            None,
                            format!("Sent you a direct message on {server}"),
                        )
                    };

                    self.execute(
                        &config.direct_message,
                        notification,
                        title.as_str(),
                        subtitle.as_deref(),
                        body.as_str(),
                        None,
                    );

                    config.direct_message.request_attention
                } else {
                    false
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
                            &format!("{} {description}", user.nickname()),
                            Some(format!("{channel} ({server})")).as_deref(),
                            message,
                            sound.as_deref(),
                        );

                        config.highlight.request_attention
                    } else {
                        self.execute(
                            &config.highlight,
                            notification,
                            user.nickname().as_str(),
                            None,
                            &format!("{description} in {channel} ({server})"),
                            sound.as_deref(),
                        );

                        config.highlight.request_attention
                    }
                } else {
                    false
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
                            user.nickname().as_str(),
                            Some(format!("{channel} ({server})")).as_deref(),
                            message,
                            None,
                        );

                        notification_config.request_attention
                    } else {
                        self.execute(
                            notification_config,
                            notification,
                            user.nickname().as_str(),
                            None,
                            &format!("Sent a message in {channel} ({server})"),
                            None,
                        );

                        notification_config.request_attention
                    }
                } else {
                    false
                }
            }
        };

        if request_attention {
            Some(iced::window::request_user_attention(
                window_id,
                Some(iced::window::UserAttention::Informational),
            ))
        } else {
            None
        }
    }

    fn execute(
        &mut self,
        config: &notification::Notification,
        notification: &Notification,
        title: &str,
        subtitle: Option<&str>,
        body: &str,
        sound_name: Option<&str>,
    ) {
        let now = Utc::now();
        let delay_key = notification.into();

        if self.recent_notifications.get(&delay_key).is_some_and(
            |last_notification| {
                now - last_notification
                    < TimeDelta::milliseconds(i64::from(
                        config.delay.unwrap_or(500),
                    ))
            },
        ) {
            return;
        }

        self.recent_notifications.insert(delay_key, now);

        if config.show_toast {
            toast::show(title, subtitle, body);
        }

        if let Some(sound) = sound_name
            .or(config.sound.as_deref())
            .and_then(|sound_name| self.sounds.get(sound_name))
        {
            audio::play(sound.clone());
        }
    }
}
