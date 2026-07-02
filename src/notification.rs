use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use data::audio::Sound;
use data::buffer::{self, Buffer};
use data::config::actions::NotificationAction;
use data::config::notification;
use data::target::join_targets;
use data::user::Nick;
use data::{Config, Notification, Server, User};
use iced::Task;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use self::toast::Toast;
pub use self::toast::prepare;
use crate::audio;

pub mod toast;

#[derive(Debug)]
pub enum Event {
    RequestAttention {
        buffer: Option<Buffer>,
    },
    NotificationResponse {
        action: toast::Action,
        buffer: Option<Buffer>,
    },
}

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
    Reaction,
    Reply(Box<str>),
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
            Notification::Reaction { .. } => NotificationDelayKey::Reaction,
            Notification::Reply { channel, .. } => {
                NotificationDelayKey::Reply(channel.as_normalized_str().into())
            }
        }
    }
}

pub struct Notifications {
    recent_notifications: HashMap<NotificationDelayKey, DateTime<Utc>>,
    sounds: HashMap<String, Sound>,
    sender: mpsc::Sender<Event>,
}

impl Notifications {
    pub fn new(config: &Config) -> (Self, Task<Event>) {
        let sounds = Notifications::load_sounds(config);

        let (sender, receiver) = mpsc::channel(50);

        (
            Self {
                recent_notifications: HashMap::new(),
                sounds,
                sender,
            },
            Task::stream(ReceiverStream::new(receiver)),
        )
    }

    pub fn update(&mut self, config: &Config) {
        // Load sounds from different sources.
        self.sounds = Notifications::load_sounds(config);
    }

    fn load_sounds(config: &Config) -> HashMap<String, Sound> {
        // Load sounds from different sources.
        config.notifications.load_sounds(
            config
                .highlights
                .matches
                .iter()
                .filter_map(|highlight_match| highlight_match.sound.as_deref()),
        )
    }

    pub fn notify(
        &mut self,
        config: &Config,
        notification: &Notification,
        server: &Server,
    ) {
        let (notification_config, title, subtitle, body, sound_name, buffer) =
            match notification {
                Notification::Connected => (
                    &config.notifications.connected,
                    "Connected".to_string(),
                    None,
                    server.to_string(),
                    None,
                    None,
                ),
                Notification::Disconnected => (
                    &config.notifications.disconnected,
                    "Disconnected".to_string(),
                    None,
                    server.to_string(),
                    None,
                    None,
                ),
                Notification::Reconnected => (
                    &config.notifications.reconnected,
                    "Reconnected".to_string(),
                    None,
                    server.to_string(),
                    None,
                    None,
                ),
                Notification::MonitoredOnline(targets) => (
                    &config.notifications.monitored_online,
                    if targets.len() == 1 {
                        "Monitored user is online"
                    } else {
                        "Monitored users are online"
                    }
                    .to_string(),
                    None,
                    join_targets(targets.iter().map(User::as_str).collect()),
                    None,
                    None,
                ),
                Notification::MonitoredOffline(targets) => (
                    &config.notifications.monitored_offline,
                    if targets.len() == 1 {
                        "Monitored user is offline"
                    } else {
                        "Monitored users are offline"
                    }
                    .to_string(),
                    None,
                    join_targets(targets.iter().map(Nick::as_str).collect()),
                    None,
                    None,
                ),
                Notification::FileTransferRequest {
                    nick,
                    casemapping,
                    filename,
                } => {
                    if config.notifications.file_transfer_request.should_notify(
                        &User::from(nick.clone()),
                        None,
                        server,
                        *casemapping,
                    ) {
                        let (title, subtitle, body): (
                            String,
                            Option<String>,
                            String,
                        ) = if config
                            .notifications
                            .file_transfer_request
                            .show_content
                        {
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

                        (
                            &config.notifications.file_transfer_request,
                            title,
                            subtitle,
                            body,
                            None,
                            Some(Buffer::Internal(
                                buffer::Internal::FileTransfers,
                            )),
                        )
                    } else {
                        return;
                    }
                }
                Notification::DirectMessage {
                    user,
                    casemapping,
                    message,
                } => {
                    if config.notifications.direct_message.should_notify(
                        user,
                        None,
                        server,
                        *casemapping,
                    ) {
                        let (title, subtitle, body): (
                            String,
                            Option<String>,
                            String,
                        ) = if config.notifications.direct_message.show_content
                        {
                            (
                                user.nickname().as_str().to_owned(),
                                Some(format!("{server}")),
                                message.to_owned(),
                            )
                        } else {
                            (
                                user.nickname().as_str().to_owned(),
                                None,
                                format!(
                                    "Sent you a direct message on {server}"
                                ),
                            )
                        };

                        (
                            &config.notifications.direct_message,
                            title,
                            subtitle,
                            body,
                            None,
                            Some(Buffer::Upstream(buffer::Upstream::Query(
                                server.clone(),
                                user.into(),
                            ))),
                        )
                    } else {
                        return;
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
                    if config.notifications.highlight.should_notify(
                        user,
                        Some(channel),
                        server,
                        *casemapping,
                    ) {
                        let buffer =
                            Buffer::Upstream(buffer::Upstream::Channel(
                                server.clone(),
                                channel.clone(),
                            ));

                        // Description is expected to be expanded by the calling
                        // routine when show_content is true
                        if config.notifications.highlight.show_content {
                            (
                                &config.notifications.highlight,
                                format!("{} {description}", user.nickname()),
                                Some(if cfg!(target_os = "macos") {
                                    format!("{channel} ({server})")
                                } else {
                                    format!("{channel}, {server}")
                                }),
                                message.to_owned(),
                                sound.to_owned(),
                                Some(buffer),
                            )
                        } else {
                            (
                                &config.notifications.highlight,
                                user.nickname().to_string(),
                                None,
                                format!(
                                    "{description} in {channel} ({server})"
                                ),
                                sound.to_owned(),
                                Some(buffer),
                            )
                        }
                    } else {
                        return;
                    }
                }
                Notification::Channel {
                    user,
                    channel,
                    casemapping,
                    message,
                } => {
                    let buffer = Buffer::Upstream(buffer::Upstream::Channel(
                        server.clone(),
                        channel.clone(),
                    ));

                    if let Some(channel_notifications_config) =
                        config.notifications.channels.get(channel.as_str())
                        && channel_notifications_config.should_notify(
                            user,
                            None,
                            server,
                            *casemapping,
                        )
                    {
                        if channel_notifications_config.show_content {
                            (
                                channel_notifications_config,
                                user.nickname().to_string(),
                                Some(if cfg!(target_os = "macos") {
                                    format!("{channel} ({server})")
                                } else {
                                    format!("{channel}, {server}")
                                }),
                                message.to_owned(),
                                None,
                                Some(buffer),
                            )
                        } else {
                            (
                                channel_notifications_config,
                                user.nickname().to_string(),
                                None,
                                format!(
                                    "Sent a message in {channel} ({server})"
                                ),
                                None,
                                Some(buffer),
                            )
                        }
                    } else {
                        return;
                    }
                }
                Notification::Reaction {
                    casemapping,
                    reaction,
                    message_text,
                } => {
                    let channel_option = reaction.target.clone().to_channel();
                    let channel = channel_option.as_ref();
                    let user = User::from(reaction.inner.sender.clone());
                    if config.notifications.reaction.should_notify(
                        &user,
                        channel,
                        server,
                        *casemapping,
                    ) {
                        let (react_sent_in, buffer) = match channel {
                            Some(channel) => (
                                if cfg!(target_os = "macos")
                                    || !config
                                        .notifications
                                        .reaction
                                        .show_content
                                {
                                    format!("{channel} ({server})")
                                } else {
                                    format!("{channel}, {server}")
                                },
                                Buffer::Upstream(buffer::Upstream::Channel(
                                    server.clone(),
                                    channel.clone(),
                                )),
                            ),
                            None => (
                                if cfg!(target_os = "macos")
                                    || !config
                                        .notifications
                                        .reaction
                                        .show_content
                                {
                                    format!("query ({server})")
                                } else {
                                    format!("query, {server}")
                                },
                                Buffer::Upstream(buffer::Upstream::Query(
                                    server.clone(),
                                    (&user).into(),
                                )),
                            ),
                        };

                        let (title, subtitle, body): (
                            String,
                            Option<String>,
                            String,
                        ) = if config.notifications.reaction.show_content {
                            (
                                user.nickname().to_string(),
                                Some(react_sent_in.to_string()),
                                format!(
                                    "Reacted {} to your message: {message_text}",
                                    reaction.inner.text
                                ),
                            )
                        } else {
                            (
                                user.nickname().to_string(),
                                Some(react_sent_in.to_string()),
                                "Reacted to your message".to_string(),
                            )
                        };

                        (
                            &config.notifications.reaction,
                            title,
                            subtitle,
                            body,
                            None,
                            Some(buffer),
                        )
                    } else {
                        return;
                    }
                }
                Notification::Reply {
                    user,
                    channel,
                    casemapping,
                    message,
                } => {
                    if config.notifications.highlight.should_notify(
                        user,
                        Some(channel),
                        server,
                        *casemapping,
                    ) {
                        let buffer =
                            Buffer::Upstream(buffer::Upstream::Channel(
                                server.clone(),
                                channel.clone(),
                            ));

                        if config.notifications.highlight.show_content {
                            (
                                &config.notifications.highlight,
                                format!("{} replied to you", user.nickname()),
                                Some(if cfg!(target_os = "macos") {
                                    format!("{channel} ({server})")
                                } else {
                                    format!("{channel}, {server}")
                                }),
                                message.to_owned(),
                                None,
                                Some(buffer),
                            )
                        } else {
                            (
                                &config.notifications.highlight,
                                user.nickname().to_string(),
                                None,
                                format!(
                                    "replied to you in {channel} ({server})"
                                ),
                                None,
                                Some(buffer),
                            )
                        }
                    } else {
                        return;
                    }
                }
            };

        if notification_config.request_attention {
            let sender = self.sender.clone();
            let buffer = buffer.clone();

            tokio::task::spawn(async move {
                let _ = sender.send(Event::RequestAttention { buffer }).await;
            });
        }

        self.execute(
            notification_config,
            config.actions.notification,
            notification,
            &title,
            subtitle.as_deref(),
            &body,
            sound_name.as_deref(),
            buffer,
        );
    }

    fn execute(
        &mut self,
        config: &notification::Notification,
        notification_action: NotificationAction,
        notification: &Notification,
        title: &str,
        subtitle: Option<&str>,
        body: &str,
        sound_name: Option<&str>,
        buffer: Option<Buffer>,
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
            let toast = Toast::new(
                title,
                subtitle,
                body,
                buffer.is_some(),
                notification_action,
            );

            let sender = self.sender.clone();

            tokio::task::spawn(async move {
                if let Some(action) = toast.show_and_wait_for_response().await {
                    let _ = sender
                        .send(Event::NotificationResponse { action, buffer })
                        .await;
                }
            });
        }

        if let Some(sound) = sound_name
            .or(config.sound.as_deref())
            .and_then(|sound_name| self.sounds.get(sound_name))
        {
            audio::play(sound.clone());
        }
    }
}
