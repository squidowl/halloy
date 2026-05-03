use std::string::ToString;

use chrono::{DateTime, Utc};
use data::dashboard::BufferAction;
use data::user::Nick;
use data::{
    Config, Server, User, config, ctcp, isupport, message, metadata, preview,
    target,
};
use iced::widget::{Space, button, column, container, image, row, rule};
use iced::{ContentFit, Length, Padding, mouse};

use crate::widget::{Element, context_menu, double_pass, text};
use crate::{Theme, font, theme, widget};

pub enum Context<'a> {
    User {
        server: &'a Server,
        prefix: &'a [isupport::PrefixMap],
        channel: Option<&'a target::Channel>,
        registry: &'a dyn metadata::Registry,
        avatar: Option<UserAvatar>,
        user: &'a User,
        current_user: Option<&'a User>,
    },
    Url {
        url: &'a str,
        message: Option<message::Hash>,
        msgid: Option<&'a message::Id>,
        selected_reactions: Vec<&'a str>,
        to_nick: Option<String>,
        reply_preview: Option<String>,
    },
    Timestamp(&'a DateTime<Utc>),
    NotSentMessage(&'a DateTime<Utc>, &'a message::Hash),
    Message {
        msgid: Option<&'a message::Id>,
        selected_reactions: &'a [String],
        content: &'a message::Content,
        source: &'a message::Source,
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
    Reply,
    AddReaction,
    Redact,
}

#[derive(Debug, Clone)]
pub enum UserAvatar {
    Pending,
    Loaded(std::path::PathBuf),
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
        can_send_reactions: bool,
        can_redact: bool,
        can_send_replies: bool,
    ) -> Vec<Self> {
        let mut entries = vec![];
        if can_send_replies {
            entries.push(Entry::Reply);
        }
        if can_send_reactions {
            entries.push(Entry::AddReaction);
        }
        if can_send_replies || can_send_reactions {
            entries.push(Entry::HorizontalRule);
        }
        entries.push(Entry::CopyMessage);
        if can_redact {
            entries.push(Entry::Redact);
        }
        entries
    }

    pub fn url_list(
        preview_hidden: Option<bool>,
        can_send_reactions: bool,
        can_redact: bool,
        can_send_replies: bool,
    ) -> Vec<Self> {
        let mut entries = vec![Entry::CopyUrl, Entry::OpenUrl];

        if let Some(preview_hidden) = preview_hidden {
            entries.push(Entry::HorizontalRule);
            entries.push(if preview_hidden {
                Entry::ShowPreview
            } else {
                Entry::HidePreview
            });
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

    pub fn view<'a>(
        self,
        context: Option<Context<'_>>,
        length: Length,
        config: &'a Config,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        context.map_or(row![].into(), |context| match (self, context) {
            (Entry::Whois, Context::User { server, user, .. }) => {
                let message =
                    Message::Whois(server.clone(), user.nickname().to_owned());

                menu_button(
                    "Whois".to_string(),
                    Some(message),
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

                menu_button(label, message, length, theme, config)
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

                menu_button(label, message, length, theme, config)
            }
            (Entry::SendFile, Context::User { server, user, .. }) => {
                let message = Message::SendFile(server.clone(), user.clone());

                menu_button(
                    "Send File".to_string(),
                    Some(message),
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
                    length,
                    theme,
                    config,
                )
            }
            (Entry::HidePreview, Context::Url { url, message, .. }) => {
                let message = message.map(|message| {
                    Message::HidePreview(message, url.to_string())
                });

                menu_button(
                    "Hide Preview".to_string(),
                    message,
                    length,
                    theme,
                    config,
                )
            }
            (Entry::ShowPreview, Context::Url { url, message, .. }) => {
                let message = message.map(|message| {
                    Message::ShowPreview(message, url.to_string())
                });

                menu_button(
                    "Show Preview".to_string(),
                    message,
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
                    length,
                    theme,
                    config,
                )
            }
            (Entry::CopyMessage, Context::Message { content, .. }) => {
                menu_button(
                    "Copy message".to_string(),
                    Some(Message::CopyMessage(content.text().into_owned())),
                    length,
                    theme,
                    config,
                )
            }
            (
                Entry::Reply,
                Context::Message {
                    msgid: Some(msgid),
                    source: message::Source::User(user),
                    content,
                    ..
                },
            ) => menu_button(
                "Reply".to_string(),
                Some(Message::Reply {
                    msgid: msgid.clone(),
                    to_nick: user.nickname().to_string(),
                    reply_preview: content.preview_text(),
                }),
                length,
                theme,
                config,
            ),
            (
                Entry::Reply,
                Context::Message {
                    msgid: Some(msgid),
                    source: message::Source::Action(Some(user)),
                    content,
                    ..
                },
            ) => menu_button(
                "Reply".to_string(),
                Some(Message::Reply {
                    msgid: msgid.clone(),
                    to_nick: user.nickname().to_string(),
                    reply_preview: content.preview_text(),
                }),
                length,
                theme,
                config,
            ),
            (
                Entry::AddReaction,
                Context::Message {
                    msgid: Some(msgid),
                    selected_reactions,
                    ..
                },
            ) => menu_button(
                "Add reaction".to_string(),
                Some(Message::OpenReactionModal(
                    msgid.clone(),
                    selected_reactions.to_vec(),
                )),
                length,
                theme,
                config,
            ),
            (
                Entry::Reply,
                Context::Url {
                    msgid: Some(msgid),
                    to_nick: Some(to_nick),
                    reply_preview: Some(reply_preview),
                    ..
                },
            ) => menu_button(
                "Reply".to_string(),
                Some(Message::Reply {
                    msgid: msgid.clone(),
                    to_nick,
                    reply_preview,
                }),
                length,
                theme,
                config,
            ),
            (
                Entry::AddReaction,
                Context::Url {
                    msgid: Some(msgid),
                    selected_reactions,
                    ..
                },
            ) => menu_button(
                "Add reaction".to_string(),
                Some(Message::OpenReactionModal(
                    msgid.clone(),
                    selected_reactions
                        .into_iter()
                        .map(ToString::to_string)
                        .collect(),
                )),
                length,
                theme,
                config,
            ),
            (
                Entry::Redact,
                Context::Message {
                    msgid: Some(msgid), ..
                }
                | Context::Url {
                    msgid: Some(msgid), ..
                },
            ) => menu_button(
                "Redact message".to_string(),
                Some(Message::Redact(msgid.clone())),
                length,
                theme,
                config,
            ),
            _ => row![].into(),
        })
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
    #[allow(clippy::enum_variant_names)]
    CopyMessage(String),
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
        to_nick: String,
        reply_preview: String,
    },
    LoadUserAvatar(Server, url::Url),
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
    #[allow(clippy::enum_variant_names)]
    CopyMessage(String),
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
        to_nick: String,
        reply_preview: String,
    },
    LoadUserAvatar(Server, url::Url),
}

pub fn update(message: Message) -> Event {
    match message {
        Message::Whois(server, nick) => Event::SendWhois(server, nick),
        Message::Whowas(server, nick) => Event::SendWhowas(server, nick),
        Message::Query(server, nick, buffer_action) => {
            Event::OpenQuery(server, nick, buffer_action)
        }
        Message::ToggleAccessLevel(server, target, nick, mode) => {
            Event::ToggleAccessLevel(server, target, nick, mode)
        }
        Message::SendFile(server, user) => Event::SendFile(server, user),
        Message::InsertNickname(nick) => Event::InsertNickname(nick),
        Message::CtcpRequest(command, server, nick, params) => {
            Event::CtcpRequest(command, server, nick, params)
        }
        Message::CopyUrl(url) => Event::CopyUrl(url),
        Message::CopyMessage(text) => Event::CopyMessage(text),
        Message::OpenUrl(url) => Event::OpenUrl(url),
        Message::HidePreview(message, url) => Event::HidePreview(message, url),
        Message::ShowPreview(message, url) => Event::ShowPreview(message, url),
        Message::CopyTimestamp(date_time) => Event::CopyTimestamp(date_time),
        Message::DeleteMessage(sesrver_time, hash) => {
            Event::DeleteMessage(sesrver_time, hash)
        }
        Message::ResendMessage(sesrver_time, hash) => {
            Event::ResendMessage(sesrver_time, hash)
        }
        Message::OpenReactionModal(msgid, selected_reactions) => {
            Event::OpenReactionModal(msgid, selected_reactions)
        }
        Message::Redact(msgid) => Event::RedactMessage(msgid),
        Message::Reply {
            msgid,
            to_nick,
            reply_preview,
        } => Event::Reply {
            msgid,
            to_nick,
            reply_preview,
        },
        Message::LoadUserAvatar(server, url) => {
            Event::LoadUserAvatar(server, url)
        }
    }
}

pub fn message<'a, M>(
    content: impl Into<Element<'a, M>>,
    source: &'a message::Source,
    msgid: Option<&'a message::Id>,
    selected_reactions: Vec<String>,
    can_send_replies: bool,
    can_send_reactions: bool,
    can_redact: bool,
    message_content: &'a message::Content,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, M>
where
    M: From<Message> + 'a,
{
    if matches!(source, message::Source::Internal(_)) {
        return content.into();
    }

    let entries = Entry::message_list(
        can_send_reactions && msgid.is_some(),
        can_redact && msgid.is_some(),
        can_send_replies && msgid.is_some(),
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
                        msgid,
                        selected_reactions: &selected_reactions,
                        content: message_content,
                        source,
                    }),
                    length,
                    config,
                    theme,
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
    click: &'a config::buffer::NicknameClickAction,
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
    click: &'a config::buffer::NicknameClickAction,
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
    click: &'a config::buffer::NicknameClickAction,
    entries: Vec<Entry>,
) -> Element<'a, Message> {
    let message = match click {
        data::config::buffer::NicknameClickAction::OpenQuery => Message::Query(
            server.clone(),
            target::Query::from(user),
            config.actions.buffer.click_username,
        ),
        data::config::buffer::NicknameClickAction::InsertNickname => {
            Message::InsertNickname(user.nickname().to_owned())
        }
    };

    let base = widget::button::transparent_button(content, message);
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
            )
        },
    )
    .into()
}

fn menu_button(
    content: String,
    message: Option<Message>,
    length: Length,
    theme: &Theme,
    config: &Config,
) -> Element<'static, Message> {
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
    const AVATAR_SIZE: f32 = 36.0;

    let query = target::Query::from(user);
    let avatar: Option<Element<'a, Message>> = avatar.map(|avatar| {
        let content: Element<'a, Message> = match avatar {
            UserAvatar::Loaded(path) => image(path.clone())
                .width(AVATAR_SIZE)
                .height(AVATAR_SIZE)
                .border_radius(4)
                .content_fit(ContentFit::Cover)
                .into(),
            UserAvatar::Pending => Space::new()
                .width(Length::Fixed(AVATAR_SIZE))
                .height(Length::Fixed(AVATAR_SIZE))
                .into(),
        };

        container(content)
            .width(Length::Fixed(AVATAR_SIZE))
            .height(Length::Fixed(AVATAR_SIZE))
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
            text(format!("{value} ({key})"))
                .style(theme::text::secondary)
                .font_maybe(theme::font_style::secondary(theme).map(font::get))
                .width(length)
                .into()
        });

    let mut content = column![];

    let inter_column_spacing = if let Some(avatar) = avatar {
        content = content.push(avatar);

        4
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
    url::Url::parse(avatar).ok()
}

pub fn user_avatar(
    user: &User,
    registry: &dyn metadata::Registry,
    previews: &preview::Collection,
) -> Option<UserAvatar> {
    avatar_url(user, registry).map(|url| match previews.get(&url) {
        Some(preview::State::Loaded(preview)) => {
            UserAvatar::Loaded(preview.image().path.clone())
        }
        _ => UserAvatar::Pending,
    })
}
