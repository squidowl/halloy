use std::collections::BTreeMap;

use chrono::{DateTime, TimeDelta, Utc};
use data::config::buffer::nickname::ShownStatus;
use data::config::buffer::{CondensationIcon, Dimmed};
use data::isupport::{CaseMap, PrefixMap};
use data::preview::{self, Previews};
use data::server::Server;
use data::user::{ChannelUsers, NickRef};
use data::{Config, User, message, target};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Color, Length, alignment};

use super::context_menu::{self, Context};
use super::scroll_view::LayoutMessage;
use crate::buffer::scroll_view::Message;
use crate::widget::reaction_row::{has_visible_reactions, reaction_row};
use crate::widget::{
    Element, Marker, message_content, message_marker, selectable_text, tooltip,
};
use crate::{Theme, font, icon, theme};

#[derive(Clone, Copy)]
pub enum TargetInfo<'a> {
    Channel {
        channel: &'a target::Channel,
        our_user: Option<&'a User>,
        users: Option<&'a ChannelUsers>,
    },
    Query,
}

impl<'a> TargetInfo<'a> {
    fn users(&self) -> Option<&'a ChannelUsers> {
        match self {
            TargetInfo::Channel { users, .. } => *users,
            TargetInfo::Query => None,
        }
    }

    fn our_user(&self) -> Option<&'a User> {
        match self {
            TargetInfo::Channel { our_user, .. } => *our_user,
            TargetInfo::Query => None,
        }
    }

    fn channel(&self) -> Option<&'a target::Channel> {
        match self {
            TargetInfo::Channel { channel, .. } => Some(channel),
            TargetInfo::Query => None,
        }
    }

    fn is_channel(&self) -> bool {
        matches!(self, TargetInfo::Channel { .. })
    }
}

#[derive(Clone, Copy)]
pub struct ChannelQueryLayout<'a> {
    pub config: &'a Config,
    pub chantypes: &'a [char],
    pub casemapping: CaseMap,
    pub prefix: &'a [PrefixMap],
    pub confirm_message_delivery: bool,
    pub can_send_reactions: bool,
    pub our_nick: Option<NickRef<'a>>,
    pub connected: bool,
    pub server: &'a Server,
    pub theme: &'a Theme,
    pub previews: Option<Previews<'a>>,
    pub target: TargetInfo<'a>,
}

impl<'a> ChannelQueryLayout<'a> {
    fn preview_hidden_for_url(
        &self,
        message: &data::Message,
        url: &str,
    ) -> Option<bool> {
        if !self.config.preview.is_enabled(url) {
            return None;
        }

        let parsed = url::Url::parse(url).ok()?;

        // Only offer hide/show when we actually have a loaded preview
        // for this URL in current context.
        let is_loaded = self
            .previews
            .and_then(|previews| previews.get(&parsed))
            .is_some_and(|state| matches!(state, preview::State::Loaded(_)));
        if !is_loaded {
            return None;
        }

        Some(message.hidden_urls.contains(&parsed))
    }

    fn url_entries(
        &self,
        message: &data::Message,
        link: &message::Link,
    ) -> Vec<context_menu::Entry> {
        match link {
            message::Link::Url(url) => context_menu::Entry::url_list(
                self.preview_hidden_for_url(message, url),
                self.can_send_reactions,
            ),
            _ => {
                if self.can_send_reactions {
                    vec![context_menu::Entry::AddReaction]
                } else {
                    vec![]
                }
            }
        }
    }

    fn condensation_marker(
        &self,
        expanded: bool,
        has_condensed: bool,
    ) -> Marker {
        let marker = if expanded {
            if has_condensed {
                Marker::Contract
            } else {
                Marker::None
            }
        } else if has_condensed {
            Marker::Expand
        } else {
            Marker::Dot
        };

        if !has_condensed {
            return marker;
        }

        match self.config.buffer.server_messages.condense.icon {
            CondensationIcon::None => Marker::None,
            CondensationIcon::Chevron => marker,
            CondensationIcon::Dot => Marker::Dot,
        }
    }

    fn format_timestamp(
        &self,
        message: &'a data::Message,
        hide_timestamp: bool,
    ) -> Option<Element<'a, Message>> {
        self.config
            .buffer
            .format_timestamp(&message.server_time)
            .map(|timestamp| {
                if hide_timestamp {
                    let width = font::width_from_chars(
                        timestamp.chars().count(),
                        &self.config.font,
                    );

                    return Space::new().width(width).into();
                }

                context_menu::timestamp(
                    selectable_text(timestamp)
                        .style(theme::selectable_text::timestamp)
                        .font_maybe(
                            theme::font_style::timestamp(self.theme)
                                .map(font::get),
                        ),
                    &message.server_time,
                    self.config,
                    self.theme,
                )
                .map(Message::ContextMenu)
            })
    }

    fn format_range_end_timestamp(
        &self,
        end_server_time: &'a DateTime<Utc>,
        hide_timestamp: bool,
    ) -> Option<Element<'a, Message>> {
        self.config
            .buffer
            .format_range_end_timestamp(end_server_time)
            .map(|(dash, end_timestamp)| {
                if hide_timestamp {
                    let width = font::width_from_chars(
                        dash.chars().count() + end_timestamp.chars().count(),
                        &self.config.font,
                    );

                    return Space::new().width(width).into();
                }

                row![
                    selectable_text(dash)
                        .style(theme::selectable_text::timestamp)
                        .font_maybe(
                            theme::font_style::timestamp(self.theme)
                                .map(font::get),
                        ),
                    context_menu::timestamp(
                        selectable_text(end_timestamp)
                            .style(theme::selectable_text::timestamp)
                            .font_maybe(
                                theme::font_style::timestamp(self.theme)
                                    .map(font::get),
                            ),
                        end_server_time,
                        self.config,
                        self.theme,
                    )
                    .map(Message::ContextMenu),
                ]
                .into()
            })
    }

    fn format_prefixes(
        &self,
        message: &'a data::Message,
        max_prefix_width: Option<f32>,
    ) -> Option<Element<'a, Message>> {
        message.target.prefixes().map_or(
            max_prefix_width.map(|width| Space::new().width(width).into()),
            |prefixes| {
                let text = selectable_text(format!(
                    "{} ",
                    self.config
                        .buffer
                        .status_message_prefix
                        .brackets
                        .format(String::from_iter(prefixes))
                ))
                .style(theme::selectable_text::tertiary)
                .font_maybe(
                    theme::font_style::tertiary(self.theme).map(font::get),
                );

                if let Some(width) = max_prefix_width {
                    Some(
                        text.width(width)
                            .align_x(text::Alignment::Right)
                            .into(),
                    )
                } else {
                    Some(text.into())
                }
            },
        )
    }

    fn not_sent_row(
        &self,
        message: &'a data::Message,
    ) -> Option<Element<'a, Message>> {
        let not_sent = self.confirm_message_delivery
            && message.command.is_some()
            && matches!(message.direction, message::Direction::Sent)
            && Utc::now().signed_duration_since(message.server_time)
                > TimeDelta::seconds(10);

        if !not_sent {
            return None;
        }

        let font_size =
            0.85 * self.config.font.size.map_or(theme::TEXT_SIZE, f32::from);
        let icon_size = theme::line_height(&self.config.font)
            .to_absolute(font_size.into())
            .0;

        Some(
            context_menu::not_sent_message(
                button(
                    row![
                        icon::not_sent()
                            .style(|theme, status| {
                                theme::svg::error(theme, status)
                            })
                            .height(icon_size)
                            .width(Length::Shrink),
                        text(" Message failed to send")
                            .style(theme::text::error)
                            .size(font_size)
                    ]
                    .align_y(alignment::Vertical::Center),
                )
                .style(theme::button::bare)
                .padding(0),
                &message.server_time,
                &message.hash,
                message.command.is_some() && self.connected,
                self.config,
                self.theme,
            )
            .map(Message::ContextMenu),
        )
    }

    fn reaction_row(
        &self,
        message: &'a data::Message,
    ) -> Option<Element<'a, Message>> {
        if !(self.config.buffer.channel.message.show_emoji_reacts
            && has_visible_reactions(message))
        {
            return None;
        }

        let selected_reaction_texts =
            selected_reactions(message, self.our_nick);
        let mut on_react = None;
        let mut on_unreact = None;
        let mut on_open_picker = None;

        if let Some(msgid) = message.id.as_ref() {
            on_react = Some(|text: &'a str| Message::Reacted {
                msgid: msgid.clone(),
                text: text.to_owned().into(),
            });
            on_unreact = Some(|text: &'a str| Message::Unreacted {
                msgid: msgid.clone(),
                text: text.to_owned().into(),
            });

            if self.can_send_reactions {
                on_open_picker = Some(Message::ContextMenu(
                    context_menu::Message::OpenReactionModal(
                        msgid.clone(),
                        selected_reaction_texts.clone(),
                    ),
                ));
            }
        }

        Some(reaction_row(
            message,
            self.our_nick,
            self.config.font.size.map_or(theme::TEXT_SIZE, f32::from),
            self.config.buffer.channel.message.max_reaction_display,
            on_react,
            on_unreact,
            on_open_picker,
        ))
    }

    fn format_user_message(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
        user: &'a User,
        hide_nickname: bool,
    ) -> (
        Element<'a, Message>,
        Element<'a, Message>,
        Vec<Element<'a, Message>>,
    ) {
        let not_sent_row = self.not_sent_row(message);

        let dimmed = not_sent_row.is_some().then_some(Dimmed::new(None));
        let dimmed_background_tuple = dimmed
            .map(|dimmed| (dimmed, self.theme.styles().buffer.background));

        let with_access_levels = self.config.buffer.nickname.show_access_levels;
        let truncate = self.config.buffer.nickname.truncate;
        let user_in_channel = self
            .target
            .users()
            .into_iter()
            .flatten()
            .find(|current_user| *current_user == user);
        let is_user_offline = match self.config.buffer.nickname.shown_status {
            ShownStatus::Current => {
                self.target.is_channel() && user_in_channel.is_none()
            }
            ShownStatus::Historical => false,
        };

        let nickname_style = theme::selectable_text::dimmed(
            theme::selectable_text::nickname(
                self.theme,
                self.config,
                match self.config.buffer.nickname.shown_status {
                    ShownStatus::Current => user_in_channel.unwrap_or(user),
                    ShownStatus::Historical => user,
                },
                is_user_offline,
            ),
            self.theme,
            dimmed_background_tuple,
        );

        let (user_display, show_nickname_tooltip) =
            user.display_with_truncated(with_access_levels, truncate);

        let nick_text =
            self.config.buffer.nickname.brackets.format(user_display);

        let nick_element: Element<_> = if hide_nickname {
            let width = match self.config.buffer.nickname.alignment {
                data::buffer::Alignment::Left
                | data::buffer::Alignment::Top => font::width_from_chars(
                    nick_text.chars().count(),
                    &self.config.font,
                ),
                data::buffer::Alignment::Right => {
                    right_aligned_width.unwrap_or_default()
                }
            };
            Space::new().width(width).into()
        } else {
            let mut nick_text = selectable_text(nick_text)
                .style(move |_| nickname_style)
                .font_maybe(
                    theme::font_style::nickname(self.theme, is_user_offline)
                        .map(font::get),
                );

            if let Some(width) = right_aligned_width {
                nick_text =
                    nick_text.width(width).align_x(text::Alignment::Right);
            }

            tooltip(
                context_menu::user(
                    nick_text,
                    self.server,
                    self.prefix,
                    self.target.channel(),
                    user,
                    user_in_channel,
                    self.target.our_user(),
                    self.config,
                    self.theme,
                    &self.config.buffer.nickname.click,
                )
                .map(Message::ContextMenu),
                show_nickname_tooltip.then_some(user.as_str()),
                tooltip::Position::Bottom,
                self.theme,
            )
        };

        let formatter = *self;

        let message_style = move |message_theme: &Theme| {
            theme::selectable_text::dimmed(
                theme::selectable_text::default(message_theme),
                message_theme,
                dimmed_background_tuple,
            )
        };

        let color_transformation = dimmed.map(|dimmed| {
            move |color: Color| -> Color {
                dimmed.transform_color(
                    color,
                    formatter.theme.styles().buffer.background,
                )
            }
        });

        let message_content = message_content::with_context(
            &message.content,
            self.server,
            self.chantypes,
            self.casemapping,
            self.theme,
            Message::Link,
            None,
            message_style,
            theme::font_style::primary,
            color_transformation,
            move |link| match link {
                message::Link::User(_, _) => context_menu::Entry::user_list(
                    formatter.target.is_channel(),
                    user_in_channel,
                    formatter.target.our_user(),
                    formatter.config.file_transfer.enabled,
                ),
                message::Link::Url(_) => formatter.url_entries(message, link),
                _ => vec![],
            },
            move |link, entry, length| {
                entry
                    .view(
                        formatter.link_context(message, link),
                        length,
                        formatter.config,
                        formatter.theme,
                    )
                    .map(Message::ContextMenu)
            },
            self.config,
        );

        let after_content =
            self.reaction_row(message).into_iter().chain(not_sent_row);

        (nick_element, message_content, after_content.collect())
    }

    fn format_server_message(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
        server: Option<&'a message::source::Server>,
    ) -> (
        Element<'a, Message>,
        Element<'a, Message>,
        Vec<Element<'a, Message>>,
    ) {
        let formatter = *self;

        let dimmed = formatter
            .config
            .buffer
            .server_messages
            .dimmed(server.map(message::source::Server::kind));

        let message_style = move |message_theme: &Theme| {
            theme::selectable_text::dimmed(
                theme::selectable_text::server(message_theme, server),
                message_theme,
                dimmed.map(|dimmed| {
                    (*dimmed, formatter.theme.styles().buffer.background)
                }),
            )
        };
        let message_font_style = move |message_theme: &Theme| {
            theme::font_style::server(message_theme, server)
        };

        let link = message.expanded.then_some(
            message::Link::ContractCondensedMessage(
                message.server_time,
                message.hash,
            ),
        );

        let marker_style = move |message_theme: &Theme| {
            if message.expanded || message.condensed.is_some() {
                theme::selectable_text::condensed_marker(message_theme)
            } else {
                message_style(message_theme)
            }
        };

        let marker = message_marker(
            self.condensation_marker(
                message.expanded,
                message.condensed.is_some(),
            ),
            right_aligned_width,
            self.config,
            marker_style,
            link.clone().map(Message::Link),
        );

        let message_content = message_content::with_context(
            &message.content,
            formatter.server,
            formatter.chantypes,
            formatter.casemapping,
            self.theme,
            Message::Link,
            link,
            message_style,
            message_font_style,
            Some(|color: Color| -> Color {
                if let Some(dimmed) = dimmed {
                    dimmed.transform_color(
                        color,
                        formatter.theme.styles().buffer.background,
                    )
                } else {
                    color
                }
            }),
            move |link| match link {
                message::Link::User(_, user) => {
                    let user_in_channel = formatter
                        .target
                        .users()
                        .into_iter()
                        .flatten()
                        .find(|u| *u == user);

                    context_menu::Entry::user_list(
                        formatter.target.is_channel(),
                        user_in_channel,
                        formatter.target.our_user(),
                        formatter.config.file_transfer.enabled,
                    )
                }
                message::Link::Url(_) => formatter.url_entries(message, link),
                _ => vec![],
            },
            move |link, entry, length| {
                entry
                    .view(
                        formatter.link_context(message, link),
                        length,
                        formatter.config,
                        formatter.theme,
                    )
                    .map(Message::ContextMenu)
            },
            self.config,
        );

        (
            marker,
            message_content,
            formatter.reaction_row(message).into_iter().collect(),
        )
    }

    fn format_condensed_message(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
        hide_timestamp: bool,
    ) -> (
        Element<'a, Message>,
        Element<'a, Message>,
        Vec<Element<'a, Message>>,
    ) {
        let formatter = *self;

        let dimmed = formatter.config.buffer.server_messages.condense.dimmed;

        let message_style = move |message_theme: &Theme| {
            theme::selectable_text::dimmed(
                theme::selectable_text::server(message_theme, None),
                message_theme,
                dimmed.map(|dimmed| {
                    (dimmed, formatter.theme.styles().buffer.background)
                }),
            )
        };
        let message_font_style = move |message_theme: &Theme| {
            theme::font_style::server(message_theme, None)
        };

        let link = message::Link::ExpandCondensedMessage(
            message.server_time,
            message.hash,
        );
        let moved_link = link.clone();

        let range_end_timestamp = if let message::Source::Internal(
            message::source::Internal::Condensed(end_server_time),
        ) = message.target.source()
            && message.server_time != *end_server_time
        {
            formatter
                .format_range_end_timestamp(end_server_time, hide_timestamp)
        } else {
            None
        };

        let condensation_marker = self.condensation_marker(false, true);
        let marker = message_marker(
            condensation_marker,
            None,
            self.config,
            theme::selectable_text::condensed_marker,
            Some(Message::Link(link.clone())),
        );

        let middle = row![
            range_end_timestamp,
            if hide_timestamp || matches!(condensation_marker, Marker::None) {
                let width = font::width_from_chars(1, &self.config.font);

                Element::from(Space::new().width(width))
            } else {
                Element::from(selectable_text(" "))
            },
            if right_aligned_width.is_some() {
                container(marker)
                    .width(Length::Fill)
                    .align_x(text::Alignment::Right)
                    .into()
            } else {
                marker
            }
        ];

        let message_content = message_content::with_context(
            &message.content,
            formatter.server,
            formatter.chantypes,
            formatter.casemapping,
            self.theme,
            move |_| Message::Link(moved_link.clone()),
            Some(link),
            message_style,
            message_font_style,
            Some(|color: Color| -> Color {
                if let Some(dimmed) = dimmed {
                    dimmed.transform_color(
                        color,
                        formatter.theme.styles().buffer.background,
                    )
                } else {
                    color
                }
            }),
            move |link| match link {
                message::Link::User(_, user) => {
                    let user_in_channel = formatter
                        .target
                        .users()
                        .into_iter()
                        .flatten()
                        .find(|u| *u == user);

                    context_menu::Entry::user_list(
                        formatter.target.is_channel(),
                        user_in_channel,
                        formatter.target.our_user(),
                        formatter.config.file_transfer.enabled,
                    )
                }
                message::Link::Url(_) => formatter.url_entries(message, link),
                _ => vec![],
            },
            move |link, entry, length| {
                entry
                    .view(
                        formatter.link_context(message, link),
                        length,
                        formatter.config,
                        formatter.theme,
                    )
                    .map(Message::ContextMenu)
            },
            self.config,
        );

        (
            if let Some(right_aligned_width) = right_aligned_width {
                container(middle).width(right_aligned_width).into()
            } else {
                middle.into()
            },
            container(message_content).into(),
            vec![],
        )
    }

    fn content_on_new_line(&self, message: &data::Message) -> bool {
        use data::buffer::Alignment;
        use message::Source;
        matches!(
            (
                message.target.source(),
                self.config.buffer.nickname.alignment,
            ),
            (Source::User(_), Alignment::Top)
        )
    }

    fn link_context<'b>(
        &'b self,
        message: &'b data::Message,
        link: &'b message::Link,
    ) -> Option<Context<'b>> {
        if let Some(user) = link.user() {
            let current_user =
                self.target.users().and_then(|users| users.resolve(user));

            Some(Context::User {
                server: self.server,
                prefix: self.prefix,
                channel: self.target.channel(),
                user,
                current_user,
            })
        } else {
            let selected_reaction_texts =
                selected_reactions_refs(message, self.our_nick);

            link.url().map(move |url| Context::Url {
                url,
                message: Some(message.hash),
                msgid: message.id.as_ref(),
                selected_reactions: selected_reaction_texts,
            })
        }
    }
}

impl<'a> LayoutMessage<'a> for ChannelQueryLayout<'a> {
    fn format(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
        max_prefix_width: Option<f32>,
        hide_timestamp: bool,
        hide_nickname: bool,
    ) -> Option<Element<'a, Message>> {
        let prefixes = self.format_prefixes(message, max_prefix_width);

        let timestamp = self.format_timestamp(message, hide_timestamp);

        let left_is_hidden = prefixes.is_none() && hide_timestamp;

        let (middle, content, after_content): (
            Element<'a, Message>,
            Element<'a, Message>,
            Vec<Element<'a, Message>>,
        ) = match message.target.source() {
            message::Source::User(user) => Some(self.format_user_message(
                message,
                right_aligned_width,
                user,
                hide_nickname,
            )),
            message::Source::Server(server_message) => {
                Some(self.format_server_message(
                    message,
                    right_aligned_width,
                    server_message.as_ref(),
                ))
            }
            message::Source::Action(_) => {
                let not_sent_row = self.not_sent_row(message);

                let dimmed =
                    not_sent_row.is_some().then_some(Dimmed::new(None));
                let dimmed_background_tuple = dimmed.map(|dimmed| {
                    (dimmed, self.theme.styles().buffer.background)
                });

                let formatter = *self;

                let message_style = move |message_theme: &Theme| {
                    theme::selectable_text::dimmed(
                        theme::selectable_text::action(message_theme),
                        message_theme,
                        dimmed_background_tuple,
                    )
                };

                let color_transformation = dimmed.map(|dimmed| {
                    move |color: Color| -> Color {
                        dimmed.transform_color(
                            color,
                            formatter.theme.styles().buffer.background,
                        )
                    }
                });

                let marker = message_marker(
                    Marker::Dot,
                    right_aligned_width,
                    self.config,
                    message_style,
                    None,
                );

                let message_content = message_content::with_context(
                    &message.content,
                    formatter.server,
                    formatter.chantypes,
                    formatter.casemapping,
                    formatter.theme,
                    Message::Link,
                    None,
                    message_style,
                    theme::font_style::action,
                    color_transformation,
                    move |link| match link {
                        message::Link::User(_, user) => {
                            let user_in_channel = formatter
                                .target
                                .users()
                                .into_iter()
                                .flatten()
                                .find(|u| *u == user);

                            context_menu::Entry::user_list(
                                formatter.target.is_channel(),
                                user_in_channel,
                                formatter.target.our_user(),
                                formatter.config.file_transfer.enabled,
                            )
                        }
                        message::Link::Url(_) => {
                            formatter.url_entries(message, link)
                        }
                        _ => vec![],
                    },
                    move |link, entry, length| {
                        entry
                            .view(
                                formatter.link_context(message, link),
                                length,
                                formatter.config,
                                formatter.theme,
                            )
                            .map(Message::ContextMenu)
                    },
                    formatter.config,
                );

                let after_content =
                    self.reaction_row(message).into_iter().chain(not_sent_row);

                Some((marker, message_content, after_content.collect()))
            }
            message::Source::Internal(message::source::Internal::Status(
                status,
            )) => {
                let message_style = move |message_theme: &Theme| {
                    theme::selectable_text::status(message_theme, *status)
                };
                let message_font_style = move |message_theme: &Theme| {
                    theme::font_style::status(message_theme, *status)
                };

                let marker = message_marker(
                    Marker::Dot,
                    right_aligned_width,
                    self.config,
                    message_style,
                    None,
                );

                let message = message_content(
                    &message.content,
                    self.server,
                    self.chantypes,
                    self.casemapping,
                    self.theme,
                    Message::Link,
                    None,
                    message_style,
                    message_font_style,
                    Option::<fn(Color) -> Color>::None,
                    self.config,
                );

                Some((marker, message, vec![]))
            }
            message::Source::Internal(message::source::Internal::Logs(_)) => {
                None
            }
            message::Source::Internal(
                message::source::Internal::Condensed(_),
            ) => (!message.text().is_empty()).then_some(
                self.format_condensed_message(
                    message,
                    right_aligned_width,
                    hide_timestamp,
                ),
            ),
        }?;

        let selected_reaction_texts =
            selected_reactions(message, self.our_nick);
        let content = context_menu::message(
            content,
            message.target.source(),
            message.id.as_ref(),
            selected_reaction_texts,
            self.can_send_reactions,
            &message.content,
            self.config,
            self.theme,
        );

        let content = if after_content.is_empty() {
            content
        } else {
            column![content].extend(after_content).into()
        };

        let row = row![
            prefixes,
            timestamp,
            if left_is_hidden {
                let width = font::width_from_chars(1, &self.config.font);

                Element::from(Space::new().width(width))
            } else {
                Element::from(selectable_text(" "))
            },
            middle,
            selectable_text(" ")
        ];

        if self.content_on_new_line(message) {
            Some(container(column![row, content]).into())
        } else {
            Some(container(row![row, content]).into())
        }
    }
}

fn selected_reactions(
    message: &data::Message,
    our_nick: Option<NickRef<'_>>,
) -> Vec<String> {
    selected_reactions_refs(message, our_nick)
        .into_iter()
        .map(ToString::to_string)
        .collect()
}

fn selected_reactions_refs<'a>(
    message: &'a data::Message,
    our_nick: Option<NickRef<'_>>,
) -> Vec<&'a str> {
    let Some(our_nick) = our_nick else {
        return vec![];
    };

    let mut selected = BTreeMap::new();

    for reaction in &message.reactions {
        if reaction.sender.as_str() == our_nick.as_str() {
            let count = selected.entry(reaction.text.as_str()).or_insert(0i16);
            if reaction.unreact {
                *count -= 1;
            } else {
                *count += 1;
            }
        }
    }

    selected
        .into_iter()
        .filter_map(|(text, count)| (count >= 1).then_some(text))
        .collect()
}
