use std::str::FromStr;
use std::string::ToString;

use chrono::{DateTime, Utc};
use data::config::actions::NicknameClickAction;
use data::dashboard::BufferAction;
use data::user::Nick;
use data::{
    Config, Server, User, ctcp, isupport, message, metadata, preview, target,
};
use iced::widget::{Space, button, center, column, container, row, rule, span};
use iced::{
    Background, Border, Color, ContentFit, Length, Padding, alignment, mouse,
};
use url::Url;

use crate::widget::{
    Element, Renderer, color_dot, context_menu, double_pass, image,
    selectable_rich_text, selectable_text, text,
};
use crate::{Theme, font, icon, theme, widget};

const AVATAR_SIZE: u16 = 36;

pub enum Context<'a> {
    User {
        server: &'a Server,
        prefix: &'a [isupport::PrefixMap],
        channel: Option<&'a target::Channel>,
        registry: &'a dyn metadata::Registry,
        avatar: Option<UserAvatar<'a>>,
        user: &'a User,
        current_user: Option<&'a User>,
    },
    Url {
        url: &'a str,
        message: Option<&'a message::Message>,
        selected_reactions: Vec<&'a str>,
    },
    Timestamp(&'a DateTime<Utc>),
    NotSentMessage(&'a DateTime<Utc>, &'a message::Hash),
    Message {
        message: &'a message::Message,
        selected_reactions: &'a [String],
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Entry {
    // user context
    Whois,
    Whowas,
    Query,
    ToggleAccessLevelOp,
    ToggleAccessLevelVoice,
    SendFile,
    UserInfo,
    UserMetadata,
    HorizontalRule,
    CtcpRequestTime,
    CtcpRequestVersion,
    // url context
    CopyUrl,
    OpenUrl,
    HidePreview,
    ShowPreview,
    // timestamp context
    Timestamp,
    // not sent message context
    DeleteMessage,
    ResendMessage,
    // message context
    CopyMessage,
    CopyRedaction,
    Reply,
    AddReaction,
    Redact,
    HideWithRedaction,
    ShowRedactedMessage,
}

impl From<super::input_view::FocusAction> for Entry {
    fn from(action: super::input_view::FocusAction) -> Self {
        use super::input_view::FocusAction;

        match action {
            FocusAction::CopyText => Entry::CopyMessage,
            FocusAction::CopyUrl => Entry::CopyUrl,
            FocusAction::Reply => Entry::Reply,
            FocusAction::OpenReactionModal => Entry::AddReaction,
            FocusAction::Redact => Entry::Redact,
            FocusAction::OpenUrl => Entry::OpenUrl,
        }
    }
}

#[derive(Debug, Clone)]
pub enum UserAvatar<'a> {
    Pending,
    Loaded(&'a data::Image),
}

impl Entry {
    pub fn not_sent_message_list(can_resend: bool) -> Vec<Self> {
        if can_resend {
            vec![Entry::DeleteMessage, Entry::ResendMessage]
        } else {
            vec![Entry::DeleteMessage]
        }
    }

    pub fn timestamp_list() -> Vec<Self> {
        vec![Entry::Timestamp]
    }

    pub fn message_list(
        has_redaction: bool,
        redaction_expanded: Option<bool>,
        can_send_reactions: bool,
        can_redact: bool,
        can_send_replies: bool,
    ) -> Vec<Self> {
        let mut entries = vec![];

        if let Some(redaction_expanded) = redaction_expanded {
            entries.push(if redaction_expanded {
                Entry::HideWithRedaction
            } else {
                Entry::ShowRedactedMessage
            });

            entries.push(Entry::HorizontalRule);
        }

        entries.push(Entry::CopyMessage);
        if has_redaction {
            entries.push(Entry::CopyRedaction);
        }

        if can_send_replies || can_send_reactions || can_redact {
            entries.push(Entry::HorizontalRule);
        }

        if can_send_replies {
            entries.push(Entry::Reply);
        }

        if can_send_reactions {
            entries.push(Entry::AddReaction);
        }

        if can_redact {
            entries.push(Entry::Redact);
        }

        entries
    }

    pub fn url_list(
        has_redaction: bool,
        redaction_expanded: Option<bool>,
        preview_hidden: Option<bool>,
        can_send_reactions: bool,
        can_redact: bool,
        can_send_replies: bool,
    ) -> Vec<Self> {
        let mut entries = vec![];

        if let Some(redaction_expanded) = redaction_expanded {
            entries.push(if redaction_expanded {
                Entry::HideWithRedaction
            } else {
                Entry::ShowRedactedMessage
            });
            entries.push(Entry::HorizontalRule);
        }

        entries.push(Entry::CopyUrl);
        entries.push(Entry::OpenUrl);

        if let Some(preview_hidden) = preview_hidden {
            entries.push(Entry::HorizontalRule);
            entries.push(if preview_hidden {
                Entry::ShowPreview
            } else {
                Entry::HidePreview
            });
        }

        entries.push(Entry::HorizontalRule);
        entries.push(Entry::CopyMessage);
        if has_redaction {
            entries.push(Entry::CopyRedaction);
        }

        if can_send_replies || can_send_reactions || can_redact {
            entries.push(Entry::HorizontalRule);
        }

        if can_send_replies {
            entries.push(Entry::Reply);
        }

        if can_send_reactions {
            entries.push(Entry::AddReaction);
        }

        if can_redact {
            entries.push(Entry::Redact);
        }

        entries
    }

    pub fn user_list(
        is_channel: bool,
        user_in_channel: Option<&User>,
        our_user: Option<&User>,
        file_transfer_enabled: bool,
        has_metadata: bool,
    ) -> Vec<Self> {
        let mut user_info_entries = vec![Entry::UserInfo];

        if has_metadata {
            user_info_entries.push(Entry::HorizontalRule);
            user_info_entries.push(Entry::UserMetadata);
        }

        if is_channel {
            if user_in_channel.is_none() {
                let mut list = user_info_entries;
                list.extend([Entry::HorizontalRule, Entry::Whowas]);
                list
            } else if our_user.is_some_and(|u| {
                u.has_access_level(data::user::AccessLevel::Oper)
            }) {
                let mut list = user_info_entries;
                list.extend([
                    Entry::HorizontalRule,
                    Entry::Whois,
                    Entry::Query,
                ]);

                if file_transfer_enabled {
                    list.push(Entry::SendFile);
                }

                list.extend(vec![
                    Entry::HorizontalRule,
                    Entry::ToggleAccessLevelOp,
                    Entry::ToggleAccessLevelVoice,
                    Entry::HorizontalRule,
                    Entry::CtcpRequestVersion,
                    Entry::CtcpRequestTime,
                ]);

                list
            } else {
                let mut list = user_info_entries;
                list.extend([
                    Entry::HorizontalRule,
                    Entry::Whois,
                    Entry::Query,
                ]);

                if file_transfer_enabled {
                    list.push(Entry::SendFile);
                }

                list.extend(vec![
                    Entry::HorizontalRule,
                    Entry::CtcpRequestVersion,
                    Entry::CtcpRequestTime,
                ]);

                list
            }
        } else {
            // In a query, server notice, or WALLOPS scenario we don't know
            // whether the user is online or not
            let mut list = vec![Entry::Whois, Entry::Whowas];

            if file_transfer_enabled {
                list.push(Entry::SendFile);
            }

            list
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Entry::CopyMessage => "Copy message",
            Entry::CopyRedaction => "Copy redaction",
            Entry::Reply => "Reply",
            Entry::AddReaction => "Add reaction",
            Entry::Redact => "Redact message",
            Entry::HideWithRedaction => "Hide with redaction",
            Entry::ShowRedactedMessage => "Show redacted message",
            _ => "",
        }
    }

    pub fn view<'a>(
        self,
        context: Option<Context<'_>>,
        length: Length,
        config: &'a Config,
        theme: &'a Theme,
        selected: bool,
    ) -> Element<'a, Message> {
        context.map_or(row![].into(), |context| match (self, context) {
            (Entry::Whois, Context::User { server, user, .. }) => {
                let message =
                    Message::Whois(server.clone(), user.nickname().to_owned());

                menu_button(
                    "Whois".to_string(),
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (Entry::Whowas, Context::User { server, user, .. }) => {
                let message =
                    Message::Whowas(server.clone(), user.nickname().to_owned());

                menu_button(
                    "Whowas".to_string(),
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (Entry::Query, Context::User { server, user, .. }) => {
                let message = Message::Query(
                    server.clone(),
                    target::Query::from(user.clone()),
                    config.actions.buffer.message_user,
                );

                menu_button(
                    "Message".to_string(),
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (
                Entry::ToggleAccessLevelOp,
                Context::User {
                    server,
                    prefix,
                    channel,
                    user,
                    ..
                },
            ) => {
                let operator_mode = prefix.iter().find_map(|prefix_map| {
                    (prefix_map.prefix == '@').then_some(prefix_map.mode)
                });

                let (label, message) =
                    if let (Some(channel), Some(operator_mode)) =
                        (channel, operator_mode)
                    {
                        let is_op = user
                            .has_access_level(data::user::AccessLevel::Oper);
                        let prefix = if is_op { "-" } else { "+" };
                        let action = format!("{prefix}{operator_mode}");

                        (
                            format!(
                                "{} Op ({action})",
                                if is_op { "Take" } else { "Give" }
                            ),
                            Some(Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                user.nickname().to_owned(),
                                action,
                            )),
                        )
                    } else {
                        (String::new(), None)
                    };

                menu_button(label, message, selected, length, theme, config)
            }
            (
                Entry::ToggleAccessLevelVoice,
                Context::User {
                    server,
                    prefix,
                    channel,
                    user,
                    ..
                },
            ) => {
                let voice_mode = prefix.iter().find_map(|prefix_map| {
                    (prefix_map.prefix == '+').then_some(prefix_map.mode)
                });

                let (label, message) =
                    if let (Some(channel), Some(voice_mode)) =
                        (channel, voice_mode)
                    {
                        let has_voice = user
                            .has_access_level(data::user::AccessLevel::Voice);
                        let prefix = if has_voice { "-" } else { "+" };
                        let action = format!("{prefix}{voice_mode}");

                        (
                            format!(
                                "{} Voice ({action})",
                                if has_voice { "Take" } else { "Give" }
                            ),
                            Some(Message::ToggleAccessLevel(
                                server.clone(),
                                channel.clone(),
                                user.nickname().to_owned(),
                                action,
                            )),
                        )
                    } else {
                        (String::new(), None)
                    };

                menu_button(label, message, selected, length, theme, config)
            }
            (Entry::SendFile, Context::User { server, user, .. }) => {
                let message = Message::SendFile(server.clone(), user.clone());

                menu_button(
                    "Send File".to_string(),
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (
                Entry::UserInfo,
                Context::User {
                    user, current_user, ..
                },
            ) => user_info(
                current_user,
                user.nickname().to_owned(),
                length,
                config,
                theme,
            ),
            (
                Entry::UserMetadata,
                Context::User {
                    user,
                    registry,
                    avatar,
                    ..
                },
            ) => user_metadata(
                user,
                registry,
                avatar.as_ref(),
                config,
                theme,
                length,
            ),
            (Entry::HorizontalRule, _) => match length {
                Length::Fill => {
                    container(rule::horizontal(1)).padding([0, 6]).into()
                }
                _ => Space::new().width(length).height(1).into(),
            },
            (Entry::CtcpRequestTime, Context::User { server, user, .. }) => {
                let message = Message::CtcpRequest(
                    ctcp::Command::Time,
                    server.clone(),
                    user.nickname().to_owned(),
                    None,
                );

                menu_button(
                    "Local Time (TIME)".to_string(),
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (Entry::CtcpRequestVersion, Context::User { server, user, .. }) => {
                let message = Message::CtcpRequest(
                    ctcp::Command::Version,
                    server.clone(),
                    user.nickname().to_owned(),
                    None,
                );

                menu_button(
                    "Client (VERSION)".to_string(),
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (Entry::CopyUrl, Context::Url { url, .. }) => {
                let message = Message::CopyUrl(url.to_string());

                menu_button(
                    "Copy URL".to_string(),
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (Entry::OpenUrl, Context::Url { url, .. }) => {
                let message = Message::OpenUrl(url.to_string());

                menu_button(
                    "Open URL".to_string(),
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (Entry::HidePreview, Context::Url { url, message, .. }) => {
                let message = message.map(|message| {
                    Message::HidePreview(message.hash, url.to_string())
                });

                menu_button(
                    "Hide Preview".to_string(),
                    message,
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (Entry::ShowPreview, Context::Url { url, message, .. }) => {
                let message = message.map(|message| {
                    Message::ShowPreview(message.hash, url.to_string())
                });

                menu_button(
                    "Show Preview".to_string(),
                    message,
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (Entry::Timestamp, Context::Timestamp(date_time)) => {
                let context_menu_timestamp =
                    config.buffer.format_context_menu_timestamp(date_time);

                let message = Message::CopyTimestamp(*date_time);

                menu_button(
                    context_menu_timestamp,
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (
                Entry::DeleteMessage,
                Context::NotSentMessage(server_time, hash),
            ) => {
                let message = Message::DeleteMessage(*server_time, *hash);

                menu_button(
                    "Delete Message".to_string(),
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (
                Entry::ResendMessage,
                Context::NotSentMessage(server_time, hash),
            ) => {
                let message = Message::ResendMessage(*server_time, *hash);

                menu_button(
                    "Re-send Message".to_string(),
                    Some(message),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (entry @ Entry::CopyMessage, Context::Message { message, .. }) => {
                menu_button(
                    entry.label().to_string(),
                    Some(Message::CopyText(message.text().into_owned())),
                    selected,
                    length,
                    theme,
                    config,
                )
            }
            (
                entry @ Entry::CopyRedaction,
                Context::Message { message, .. }
                | Context::Url {
                    message: Some(message),
                    ..
                },
            ) => {
                if let Some(redaction) = message.redaction.as_ref() {
                    menu_button(
                        entry.label().to_string(),
                        Some(Message::CopyText(redaction.message())),
                        selected,
                        length,
                        theme,
                        config,
                    )
                } else {
                    row![].into()
                }
            }
            (
                entry @ Entry::Reply,
                Context::Message { message, .. }
                | Context::Url {
                    message: Some(message),
                    ..
                },
            ) => {
                if let Some(msgid) = message.id.as_ref()
                    && let Some(user) = message.target.source().user()
                {
                    menu_button(
                        entry.label().to_string(),
                        Some(Message::Reply {
                            msgid: msgid.clone(),
                            server_time: message.server_time,
                            to_nick: user.nickname().to_owned(),
                        }),
                        selected,
                        length,
                        theme,
                        config,
                    )
                } else {
                    row![].into()
                }
            }
            (
                entry @ Entry::AddReaction,
                Context::Message {
                    message,
                    selected_reactions,
                    ..
                },
            ) => {
                if let Some(msgid) = message.id.as_ref() {
                    menu_button(
                        entry.label().to_string(),
                        Some(Message::OpenReactionModal(
                            msgid.clone(),
                            selected_reactions.to_vec(),
                        )),
                        selected,
                        length,
                        theme,
                        config,
                    )
                } else {
                    row![].into()
                }
            }
            (
                entry @ Entry::AddReaction,
                Context::Url {
                    message: Some(message),
                    selected_reactions,
                    ..
                },
            ) => {
                if let Some(msgid) = message.id.as_ref() {
                    menu_button(
                        entry.label().to_string(),
                        Some(Message::OpenReactionModal(
                            msgid.clone(),
                            selected_reactions
                                .into_iter()
                                .map(ToString::to_string)
                                .collect(),
                        )),
                        selected,
                        length,
                        theme,
                        config,
                    )
                } else {
                    row![].into()
                }
            }
            (
                entry @ Entry::Redact,
                Context::Message { message, .. }
                | Context::Url {
                    message: Some(message),
                    ..
                },
            ) => {
                if let Some(msgid) = message.id.as_ref() {
                    menu_button(
                        entry.label().to_string(),
                        Some(Message::Redact(msgid.clone())),
                        selected,
                        length,
                        theme,
                        config,
                    )
                } else {
                    row![].into()
                }
            }
            (
                entry @ Entry::HideWithRedaction,
                Context::Message { message, .. }
                | Context::Url {
                    message: Some(message),
                    ..
                },
            ) => menu_button(
                entry.label().to_string(),
                Some(Message::ContractMessage(
                    message.server_time,
                    message.hash,
                )),
                selected,
                length,
                theme,
                config,
            ),
            (
                entry @ Entry::ShowRedactedMessage,
                Context::Message { message, .. }
                | Context::Url {
                    message: Some(message),
                    ..
                },
            ) => menu_button(
                entry.label().to_string(),
                Some(Message::ExpandMessage(message.server_time, message.hash)),
                selected,
                length,
                theme,
                config,
            ),
            _ => row![].into(),
        })
    }

    pub fn context_message(
        self,
        context: &Context<'_>,
        config: &Config,
    ) -> Option<Message> {
        let &Context::User {
            server,
            prefix,
            channel,
            user,
            ..
        } = context
        else {
            return None;
        };

        match self {
            Entry::Whois => {
                Some(Message::Whois(server.clone(), user.nickname().to_owned()))
            }
            Entry::Whowas => Some(Message::Whowas(
                server.clone(),
                user.nickname().to_owned(),
            )),
            Entry::Query => Some(Message::Query(
                server.clone(),
                target::Query::from(user.clone()),
                config.actions.buffer.message_user,
            )),
            Entry::SendFile => {
                Some(Message::SendFile(server.clone(), user.clone()))
            }
            Entry::CtcpRequestTime => Some(Message::CtcpRequest(
                ctcp::Command::Time,
                server.clone(),
                user.nickname().to_owned(),
                None,
            )),
            Entry::CtcpRequestVersion => Some(Message::CtcpRequest(
                ctcp::Command::Version,
                server.clone(),
                user.nickname().to_owned(),
                None,
            )),
            Entry::ToggleAccessLevelOp => {
                let operator_mode = prefix.iter().find_map(|prefix_map| {
                    (prefix_map.prefix == '@').then_some(prefix_map.mode)
                });

                channel.zip(operator_mode).map(|(channel, operator_mode)| {
                    let is_op =
                        user.has_access_level(data::user::AccessLevel::Oper);
                    let prefix = if is_op { "-" } else { "+" };

                    Message::ToggleAccessLevel(
                        server.clone(),
                        channel.clone(),
                        user.nickname().to_owned(),
                        format!("{prefix}{operator_mode}"),
                    )
                })
            }
            Entry::ToggleAccessLevelVoice => {
                let voice_mode = prefix.iter().find_map(|prefix_map| {
                    (prefix_map.prefix == '+').then_some(prefix_map.mode)
                });

                channel.zip(voice_mode).map(|(channel, voice_mode)| {
                    let has_voice =
                        user.has_access_level(data::user::AccessLevel::Voice);
                    let prefix = if has_voice { "-" } else { "+" };

                    Message::ToggleAccessLevel(
                        server.clone(),
                        channel.clone(),
                        user.nickname().to_owned(),
                        format!("{prefix}{voice_mode}"),
                    )
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Whois(Server, Nick),
    Whowas(Server, Nick),
    Query(Server, target::Query, BufferAction),
    ToggleAccessLevel(Server, target::Channel, Nick, String),
    SendFile(Server, User),
    InsertNickname(Nick),
    CtcpRequest(ctcp::Command, Server, Nick, Option<String>),
    CopyUrl(String),
    CopyText(String),
    OpenUrl(String),
    HidePreview(message::Hash, String),
    ShowPreview(message::Hash, String),
    CopyTimestamp(DateTime<Utc>),
    #[allow(clippy::enum_variant_names)]
    DeleteMessage(DateTime<Utc>, message::Hash),
    #[allow(clippy::enum_variant_names)]
    ResendMessage(DateTime<Utc>, message::Hash),
    OpenReactionModal(message::Id, Vec<String>),
    Redact(message::Id),
    Reply {
        msgid: message::Id,
        server_time: DateTime<Utc>,
        to_nick: Nick,
    },
    LoadUserAvatar(Server, url::Url),
    Link(message::Link),
    #[allow(clippy::enum_variant_names)]
    ExpandMessage(DateTime<Utc>, message::Hash),
    #[allow(clippy::enum_variant_names)]
    ContractMessage(DateTime<Utc>, message::Hash),
}

#[derive(Debug, Clone)]
pub enum Event {
    SendWhois(Server, Nick),
    SendWhowas(Server, Nick),
    OpenQuery(Server, target::Query, BufferAction),
    ToggleAccessLevel(Server, target::Channel, Nick, String),
    SendFile(Server, User),
    InsertNickname(Nick),
    CtcpRequest(ctcp::Command, Server, Nick, Option<String>),
    CopyUrl(String),
    CopyText(String),
    OpenUrl(String),
    HidePreview(message::Hash, String),
    ShowPreview(message::Hash, String),
    CopyTimestamp(DateTime<Utc>),
    DeleteMessage(DateTime<Utc>, message::Hash),
    ResendMessage(DateTime<Utc>, message::Hash),
    OpenReactionModal(message::Id, Vec<String>),
    RedactMessage(message::Id),
    Reply {
        msgid: message::Id,
        server_time: DateTime<Utc>,
        to_nick: Nick,
    },
    LoadUserAvatar(Server, url::Url),
    ExpandMessage(DateTime<Utc>, message::Hash),
    ContractMessage(DateTime<Utc>, message::Hash),
}

pub fn update(message: Message) -> Option<Event> {
    match message {
        Message::Whois(server, nick) => Some(Event::SendWhois(server, nick)),
        Message::Whowas(server, nick) => Some(Event::SendWhowas(server, nick)),
        Message::Query(server, nick, buffer_action) => {
            Some(Event::OpenQuery(server, nick, buffer_action))
        }
        Message::ToggleAccessLevel(server, target, nick, mode) => {
            Some(Event::ToggleAccessLevel(server, target, nick, mode))
        }
        Message::SendFile(server, user) => Some(Event::SendFile(server, user)),
        Message::InsertNickname(nick) => Some(Event::InsertNickname(nick)),
        Message::CtcpRequest(command, server, nick, params) => {
            Some(Event::CtcpRequest(command, server, nick, params))
        }
        Message::CopyUrl(url) => Some(Event::CopyUrl(url)),
        Message::CopyText(text) => Some(Event::CopyText(text)),
        Message::OpenUrl(url) => Some(Event::OpenUrl(url)),
        Message::HidePreview(message, url) => {
            Some(Event::HidePreview(message, url))
        }
        Message::ShowPreview(message, url) => {
            Some(Event::ShowPreview(message, url))
        }
        Message::CopyTimestamp(date_time) => {
            Some(Event::CopyTimestamp(date_time))
        }
        Message::DeleteMessage(server_time, hash) => {
            Some(Event::DeleteMessage(server_time, hash))
        }
        Message::ResendMessage(server_time, hash) => {
            Some(Event::ResendMessage(server_time, hash))
        }
        Message::OpenReactionModal(msgid, selected_reactions) => {
            Some(Event::OpenReactionModal(msgid, selected_reactions))
        }
        Message::Redact(msgid) => Some(Event::RedactMessage(msgid)),
        Message::Reply {
            msgid,
            server_time,
            to_nick,
        } => Some(Event::Reply {
            msgid,
            server_time,
            to_nick,
        }),
        Message::LoadUserAvatar(server, url) => {
            Some(Event::LoadUserAvatar(server, url))
        }
        Message::Link(message::Link::Url(url)) => Some(Event::OpenUrl(url)),
        Message::Link(_) => None,
        Message::ExpandMessage(server_time, hash) => {
            Some(Event::ExpandMessage(server_time, hash))
        }
        Message::ContractMessage(server_time, hash) => {
            Some(Event::ContractMessage(server_time, hash))
        }
    }
}

pub fn message<'a, M>(
    content: impl Into<Element<'a, M>>,
    message: &'a message::Message,
    selected_reactions: Vec<String>,
    can_send_replies: bool,
    can_send_reactions: bool,
    can_redact: bool,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, M>
where
    M: From<Message> + 'a,
{
    if matches!(message.target.source(), message::Source::Internal(_)) {
        return content.into();
    }

    let entries = Entry::message_list(
        message.redaction.is_some(),
        message.redaction_expanded(&config.buffer.redaction),
        can_send_reactions && message.id.is_some(),
        can_redact && message.id.is_some(),
        can_send_replies
            && message.id.is_some()
            && message.rerouted_from.is_none(),
    );

    context_menu(
        context_menu::MouseButton::default(),
        context_menu::Anchor::Cursor,
        context_menu::ToggleBehavior::KeepOpen,
        None,
        content,
        entries,
        move |entry, length| {
            entry
                .view(
                    Some(Context::Message {
                        message,
                        selected_reactions: &selected_reactions,
                    }),
                    length,
                    config,
                    theme,
                    false,
                )
                .map(M::from)
        },
    )
    .into()
}

pub fn preview<'a, M>(
    content: impl Into<Element<'a, M>>,
    url: &'a str,
    can_send_replies: bool,
    can_send_reactions: bool,
    can_redact: bool,
    message: &'a message::Message,
    selected_reactions: Vec<&'a str>,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, M>
where
    M: From<Message> + 'a,
{
    let entries = Entry::url_list(
        false, // Previews are hidden if the message is redacted
        None,  // Previews are hidden if the message is redacted
        Some(false),
        can_send_reactions && message.id.is_some(),
        can_redact && message.id.is_some(),
        can_send_replies && message.id.is_some(),
    );

    context_menu(
        context_menu::MouseButton::Right,
        context_menu::Anchor::Cursor,
        context_menu::ToggleBehavior::KeepOpen,
        None,
        content,
        entries,
        move |entry, length| {
            entry
                .view(
                    Some(Context::Url {
                        url,
                        message: Some(message),
                        selected_reactions: selected_reactions.clone(),
                    }),
                    length,
                    config,
                    theme,
                    false,
                )
                .map(M::from)
        },
    )
    .into()
}

pub fn user<'a>(
    content: impl Into<Element<'a, Message>>,
    server: &'a Server,
    prefix: &'a [isupport::PrefixMap],
    channel: Option<&'a target::Channel>,
    registry: &'a dyn metadata::Registry,
    previews: &'a preview::Collection,
    user: &'a User,
    current_user: Option<&'a User>,
    our_user: Option<&'a User>,
    config: &'a Config,
    theme: &'a Theme,
    click: &'a NicknameClickAction,
) -> Element<'a, Message> {
    let entries = Entry::user_list(
        channel.is_some(),
        current_user,
        our_user,
        config.file_transfer.enabled,
        has_user_metadata(user, registry, config),
    );

    user_with_entries(
        content,
        server,
        prefix,
        channel,
        registry,
        previews,
        user,
        current_user,
        config,
        theme,
        click,
        entries,
    )
}

pub fn rerouted_private_user<'a>(
    content: impl Into<Element<'a, Message>>,
    server: &'a Server,
    prefix: &'a [isupport::PrefixMap],
    registry: &'a dyn metadata::Registry,
    previews: &'a preview::Collection,
    user: &'a User,
    config: &'a Config,
    theme: &'a Theme,
    click: &'a NicknameClickAction,
) -> Element<'a, Message> {
    user_with_entries(
        content,
        server,
        prefix,
        None,
        registry,
        previews,
        user,
        None,
        config,
        theme,
        click,
        vec![Entry::Whois],
    )
}

fn user_with_entries<'a>(
    content: impl Into<Element<'a, Message>>,
    server: &'a Server,
    prefix: &'a [isupport::PrefixMap],
    channel: Option<&'a target::Channel>,
    registry: &'a dyn metadata::Registry,
    previews: &'a preview::Collection,
    user: &'a User,
    current_user: Option<&'a User>,
    config: &'a Config,
    theme: &'a Theme,
    click: &'a NicknameClickAction,
    entries: Vec<Entry>,
) -> Element<'a, Message> {
    let message = match click {
        NicknameClickAction::OpenQuery(buffer_action) => Some(Message::Query(
            server.clone(),
            target::Query::from(user),
            *buffer_action,
        )),
        NicknameClickAction::InsertNickname => {
            Some(Message::InsertNickname(user.nickname().to_owned()))
        }
        NicknameClickAction::Noop => None,
    };

    let base = if let Some(message) = message {
        widget::button::transparent_button(content, message)
    } else {
        content.into()
    };
    let avatar = user_avatar(user, registry, previews);
    let on_open = config
        .context_menu
        .show_user_metadata
        .then(|| avatar_url(user, registry))
        .flatten()
        .filter(|url| !previews.contains_key(url))
        .map(|url| {
            let server = server.clone();

            move || Message::LoadUserAvatar(server.clone(), url.clone())
        });

    let menu = context_menu(
        context_menu::MouseButton::default(),
        context_menu::Anchor::Cursor,
        context_menu::ToggleBehavior::KeepOpen,
        Some(mouse::Interaction::Pointer),
        base,
        entries,
        move |entry, length| {
            entry.view(
                Some(Context::User {
                    server,
                    prefix,
                    channel,
                    registry,
                    avatar: avatar.clone(),
                    user,
                    current_user,
                }),
                length,
                config,
                theme,
                false,
            )
        },
    );

    if let Some(on_open) = on_open {
        menu.on_open(on_open).into()
    } else {
        menu.into()
    }
}

pub fn timestamp<'a>(
    content: impl Into<Element<'a, Message>>,
    date_time: &'a DateTime<Utc>,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let entries = Entry::timestamp_list();

    context_menu(
        context_menu::MouseButton::default(),
        context_menu::Anchor::Cursor,
        context_menu::ToggleBehavior::KeepOpen,
        None,
        content,
        entries,
        move |entry, length| {
            entry.view(
                Some(Context::Timestamp(date_time)),
                length,
                config,
                theme,
                false,
            )
        },
    )
    .into()
}

pub fn not_sent_message<'a>(
    content: impl Into<Element<'a, Message>>,
    server_time: &'a DateTime<Utc>,
    hash: &'a message::Hash,
    can_resend: bool,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let entries = Entry::not_sent_message_list(can_resend);

    context_menu(
        context_menu::MouseButton::Left,
        context_menu::Anchor::Cursor,
        context_menu::ToggleBehavior::KeepOpen,
        Some(mouse::Interaction::Pointer),
        content,
        entries,
        move |entry, length| {
            entry.view(
                Some(Context::NotSentMessage(server_time, hash)),
                length,
                config,
                theme,
                false,
            )
        },
    )
    .into()
}

pub(crate) fn menu_button<'a, M: Clone + 'a>(
    content: String,
    message: Option<M>,
    selected: bool,
    length: Length,
    theme: &Theme,
    config: &Config,
) -> Element<'a, M> {
    let text_style = if message.is_some() {
        theme::text::primary
    } else {
        theme::text::secondary
    };

    button(
        text(content)
            .style(text_style)
            .font_maybe(theme::font_style::primary(theme).map(font::get)),
    )
    .padding(config.context_menu.padding.entry)
    .width(length)
    .on_press_maybe(message)
    .style(move |theme, status| theme::button::primary(theme, status, selected))
    .into()
}

fn right_justified_padding(config: &Config) -> Padding {
    let padding = config.context_menu.padding.entry;
    Padding::from(padding)
        .right(f32::from(padding[1]) + double_pass::horizontal_expansion())
}

fn user_info<'a>(
    current_user: Option<&User>,
    nickname: Nick,
    length: Length,
    config: &Config,
    theme: &Theme,
) -> Element<'a, Message> {
    let state = match current_user {
        Some(user) => {
            if user.is_away() {
                Some(
                    text("(Away)")
                        .style(theme::text::secondary)
                        .font_maybe(
                            theme::font_style::secondary(theme).map(font::get),
                        )
                        .width(length),
                )
            } else {
                None
            }
        }
        None => Some(
            text("(Offline)")
                .style(theme::text::secondary)
                .font_maybe(theme::font_style::secondary(theme).map(font::get))
                .width(length),
        ),
    };

    // Dimmed if away or offline.
    let is_user_away = config
        .buffer
        .nickname
        .away
        .is_away(current_user.is_none_or(User::is_away));
    let is_user_offline = config
        .buffer
        .nickname
        .offline
        .is_offline(current_user.is_none());
    let style = theme::text::nickname(
        theme,
        &config.buffer.nickname.color,
        Some(nickname.seed()),
        is_user_away,
        is_user_offline,
    );

    let nickname = text(nickname.to_string()).style(move |_| style).font_maybe(
        theme::font_style::nickname(theme, is_user_offline).map(font::get),
    );

    column![
        container(row![nickname, state].width(length).spacing(4))
            .padding(right_justified_padding(config))
    ]
    .into()
}

pub fn has_user_metadata(
    user: &User,
    registry: &dyn metadata::Registry,
    config: &Config,
) -> bool {
    if !config.context_menu.show_user_metadata {
        return false;
    }

    let query = target::Query::from(user);

    config.metadata.preferred_keys.iter().copied().any(|key| {
        registry
            .get_user(&query, key)
            .is_some_and(|value| !value.is_empty())
    })
}

fn user_metadata<'a>(
    user: &User,
    registry: &dyn metadata::Registry,
    avatar: Option<&UserAvatar>,
    config: &Config,
    theme: &'a Theme,
    length: Length,
) -> Element<'a, Message> {
    let query = target::Query::from(user);
    let avatar: Option<Element<'a, Message>> = avatar.map(|avatar| {
        let content: Element<'a, Message> = match avatar {
            UserAvatar::Loaded(data) => {
                container(image::from_data(data, true, ContentFit::Cover))
                    .width(f32::from(AVATAR_SIZE))
                    .height(f32::from(AVATAR_SIZE))
                    .into()
            }
            UserAvatar::Pending => avatar_placeholder(),
        };

        container(content)
            .width(Length::Fixed(f32::from(AVATAR_SIZE)))
            .height(Length::Fixed(f32::from(AVATAR_SIZE)))
            .into()
    });
    let rows = config
        .metadata
        .preferred_keys
        .iter()
        .copied()
        .filter(|key| !matches!(key, metadata::Key::Avatar))
        .filter_map(|key| {
            registry
                .get_user(&query, key)
                .filter(|value| !value.is_empty())
                .map(|value| (key, value))
        })
        .map(|(key, value)| {
            match key {
                metadata::Key::Homepage => Url::parse(value).ok().map(|url| {
                    selectable_rich_text::<
                        Message,
                        message::Link,
                        (),
                        Theme,
                        Renderer,
                    >(vec![
                        if config.display.decode_urls {
                            span(data::url::display(&url).to_string())
                        } else {
                            span(value.to_string())
                        }
                        .color(theme.styles().buffer.url.color)
                        .link(message::Link::Url(url.as_str().to_string())),
                        span(format!(" ({key})")),
                    ])
                    .on_link(Message::Link)
                    .style(theme::selectable_text::secondary)
                    .font_maybe(
                        theme::font_style::secondary(theme).map(font::get),
                    )
                    .width(length)
                    .into()
                }),
                metadata::Key::Color => {
                    Color::from_str(value).ok().map(|color| {
                        row![
                            color_dot(color),
                            selectable_text(format!("{value} ({key})"))
                                .style(theme::selectable_text::secondary)
                                .font_maybe(
                                    theme::font_style::secondary(theme)
                                        .map(font::get),
                                )
                        ]
                        .spacing(5)
                        .align_y(alignment::Vertical::Center)
                        .width(length)
                        .into()
                    })
                }
                _ => None,
            }
            .unwrap_or(
                selectable_text(format!("{value} ({key})"))
                    .style(theme::selectable_text::secondary)
                    .font_maybe(
                        theme::font_style::secondary(theme).map(font::get),
                    )
                    .width(length)
                    .into(),
            )
        });

    let mut content = column![];

    let inter_column_spacing = if let Some(avatar) = avatar {
        content = content.push(avatar);

        6
    } else {
        0
    };

    row![content, column(rows).spacing(2)]
        .spacing(inter_column_spacing)
        .align_y(iced::alignment::Vertical::Top)
        .padding(right_justified_padding(config))
        .into()
}

fn avatar_url(
    user: &User,
    registry: &dyn metadata::Registry,
) -> Option<url::Url> {
    let query = target::Query::from(user);
    let avatar = registry.avatar(target::TargetRef::Query(&query))?;

    // Replace optional `{size}` in the avatar URL with the display size
    // https://ircv3.net/registry#user-metadata
    let sized_avatar = avatar.replace("{size}", &format!("{AVATAR_SIZE}"));

    url::Url::parse(&sized_avatar).ok()
}

pub fn user_avatar<'a>(
    user: &User,
    registry: &dyn metadata::Registry,
    previews: &'a preview::Collection,
) -> Option<UserAvatar<'a>> {
    avatar_url(user, registry).map(|url| match previews.get(&url) {
        Some(preview::State::Loaded(preview)) => {
            UserAvatar::Loaded(preview.image())
        }
        _ => UserAvatar::Pending,
    })
}

fn avatar_placeholder<'a>() -> Element<'a, Message> {
    center(icon::people().size(16).style(theme::text::secondary))
        .width(Length::Fixed(f32::from(AVATAR_SIZE)))
        .height(Length::Fixed(f32::from(AVATAR_SIZE)))
        .style(|theme| {
            let general = theme.styles().general;
            let text = theme.styles().text;

            container::Style {
                background: Some(Background::Color(general.background)),
                border: Border {
                    radius: 4.0.into(),
                    width: 0.5,
                    color: text.secondary.color,
                },
                ..Default::default()
            }
        })
        .into()
}
