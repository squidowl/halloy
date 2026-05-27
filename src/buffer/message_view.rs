use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, TimeDelta, Utc};
use data::buffer::RightAlignmentWidths;
use data::config::buffer::nickname::ShownStatus;
use data::config::buffer::{CondensationIcon, Dimmed};
use data::config::preview::HideUrlCondition;
use data::isupport::{CaseMap, PrefixMap};
use data::preview::{self, Previews};
use data::redaction::Redaction;
use data::server::Server;
use data::user::{ChannelUsers, NickRef};
use data::{Config, Preview, User, history, message, metadata, target};
use iced::widget::text::LineHeight;
use iced::widget::{
    Space, button, center, column, container, mouse_area, right, row, space,
    stack, text,
};
use iced::{Color, ContentFit, Length, alignment, padding};

use crate::buffer::context_menu::{self, Context};
use crate::buffer::scroll_view::keyed::{self, keyed};
use crate::buffer::scroll_view::{LayoutMessage, Message};
use crate::widget::preview::preview_card_parts;
use crate::widget::reaction_row::{has_visible_reactions, reaction_row};
use crate::widget::user_display::UserDisplay;
use crate::widget::{
    Element, Marker, message_content, message_marker, notify_visibility,
    preview_content, reply_preview_content, selectable_text, tooltip,
};
use crate::{Theme, font, icon, theme};

const HIDE_BUTTON_WIDTH: f32 = 22.0;

#[derive(Clone, Copy)]
pub enum TargetInfo<'a> {
    Channel {
        channel: &'a target::Channel,
        our_user: Option<&'a User>,
        users: Option<&'a ChannelUsers>,
    },
    Query {
        query: &'a target::Query,
    },
}

impl<'a> TargetInfo<'a> {
    fn users(&self) -> Option<&'a ChannelUsers> {
        match self {
            TargetInfo::Channel { users, .. } => *users,
            TargetInfo::Query { .. } => None,
        }
    }

    fn our_user(&self) -> Option<&'a User> {
        match self {
            TargetInfo::Channel { our_user, .. } => *our_user,
            TargetInfo::Query { .. } => None,
        }
    }

    fn channel(&self) -> Option<&'a target::Channel> {
        match self {
            TargetInfo::Channel { channel, .. } => Some(channel),
            TargetInfo::Query { .. } => None,
        }
    }

    fn is_channel(&self) -> bool {
        matches!(self, TargetInfo::Channel { .. })
    }

    fn as_target_ref(&self) -> target::TargetRef<'a> {
        match self {
            TargetInfo::Channel { channel, .. } => channel.as_target_ref(),
            TargetInfo::Query { query } => query.as_target_ref(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct ChannelQueryLayout<'a> {
    pub config: &'a Config,
    pub chantypes: &'a [char],
    pub casemapping: CaseMap,
    pub prefix: &'a [PrefixMap],
    pub registry: &'a dyn metadata::Registry,
    pub confirm_message_delivery: bool,
    pub can_send_replies: bool,
    pub can_send_reactions: bool,
    pub can_redact: bool,
    pub our_nick: Option<NickRef<'a>>,
    pub connected: bool,
    pub server: &'a Server,
    pub theme: &'a Theme,
    pub previews: Previews<'a>,
    pub target: TargetInfo<'a>,
    pub history: &'a history::Manager,
}

impl<'a> ChannelQueryLayout<'a> {
    fn reply_nick_to_strip<'m>(
        &self,
        message: &'m data::Message,
    ) -> Option<&'m str> {
        if self.config.buffer.reply.hide_redundant_nicks {
            message
                .reply_preview
                .as_ref()
                .and_then(|reply_preview| reply_preview.user.as_ref())
                .map(User::as_str)
        } else {
            None
        }
    }

    fn previews_enabled(&self, message: &data::Message) -> bool {
        !self.not_sent(message) && message.redaction.is_none()
    }

    fn preview_hidden_for_url(
        &self,
        message: &data::Message,
        url: &str,
    ) -> Option<bool> {
        if !self.previews_enabled(message)
            || !self.config.preview.is_enabled(url)
        {
            return None;
        }

        let parsed = url::Url::parse(url).ok()?;

        // Only offer hide/show when we actually have a loaded preview
        // for this URL in current context.
        let is_loaded = self
            .previews
            .get(&parsed)
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
        let can_send_replies = self.can_send_replies && message.id.is_some();
        match link {
            message::Link::Url(url) => context_menu::Entry::url_list(
                message.redaction.is_some(),
                message.redaction_expanded(&self.config.buffer.redaction),
                self.preview_hidden_for_url(message, url),
                self.can_send_reactions,
                self.can_redact_message(message),
                can_send_replies,
            ),
            _ => {
                let mut entries = vec![];
                if can_send_replies || self.can_send_reactions {
                    if can_send_replies {
                        entries.push(context_menu::Entry::Reply);
                    }
                    if self.can_send_reactions {
                        entries.push(context_menu::Entry::AddReaction);
                    }
                }
                entries
            }
        }
    }

    fn can_redact_message(&self, message: &data::Message) -> bool {
        // Gate on message-redaction capability first.
        if !self.can_redact {
            return false;
        }

        // A message can only be redacted once.
        if message.redaction.is_some() {
            return false;
        }

        // Message MUST be PRIVMSG, NOTICE, or TAGMSG
        match message.target.source() {
            message::Source::User(_) | message::Source::Action(_) => true,
            message::Source::Server(_) | message::Source::Internal(_) => false,
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
                    let width =
                        font::width_from_str(&timestamp, &self.config.font);

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
                    let width = font::width_from_str(
                        &format!("{dash}{end_timestamp}"),
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
    ) -> Option<Element<'a, Message>> {
        message.target.prefixes().map(|prefixes| {
            selectable_text(format!(
                "{} ",
                self.config
                    .buffer
                    .status_message_prefix
                    .brackets
                    .format(String::from_iter(prefixes))
            ))
            .style(theme::selectable_text::tertiary)
            .font_maybe(theme::font_style::tertiary(self.theme).map(font::get))
            .into()
        })
    }

    fn not_sent(&self, message: &data::Message) -> bool {
        self.confirm_message_delivery
            && message.command.is_some()
            && matches!(message.direction, message::Direction::Sent)
            && Utc::now().signed_duration_since(message.server_time)
                > TimeDelta::seconds(10)
    }

    fn not_sent_row(
        &self,
        message: &'a data::Message,
    ) -> Option<Element<'a, Message>> {
        if !self.not_sent(message) {
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
                            .style(theme::svg::error)
                            .height(icon_size)
                            .width(Length::Shrink)
                            .content_fit(ContentFit::Contain),
                        text(" Message failed to send")
                            .line_height(LineHeight::Relative(1.0))
                            .style(theme::text::error)
                            .size(font_size)
                    ]
                    .align_y(alignment::Vertical::Center),
                )
                .style(theme::button::bare)
                .padding(padding::top(self.config.buffer.line_spacing)),
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
            self.config.font.only_emojis_size.map(f32::from),
            self.config.buffer.channel.message.max_reaction_display,
            on_react,
            on_unreact,
            on_open_picker,
            self.config.tooltips.show_for_buttons(),
            self.target.users(),
            self.casemapping,
            self.config,
            self.registry,
            self.theme,
        ))
    }

    fn preview_row(
        &self,
        message: &'a data::Message,
        preview: &'a Preview,
        url: &'a url::Url,
        index: usize,
        is_hovered: bool,
    ) -> Element<'a, Message> {
        let content = match preview {
            data::Preview::Card(card) => {
                let (body, image) =
                    preview_card_parts(card, self.config, self.theme);

                let mut card_content = column![
                    button(body)
                        .on_press(Message::Link(message::Link::Url(
                            url.to_string(),
                        )))
                        .padding(0)
                        .style(theme::button::bare),
                ]
                .spacing(8)
                .max_width(self.config.preview.card.max_width);

                if let Some(image) = image {
                    card_content = card_content.push(
                        button(image)
                            .on_press(match self.config.preview.card.image_action
                            {
                                data::config::preview::CardImageAction::OpenUrl => {
                                    Message::Link(message::Link::Url(
                                        url.to_string(),
                                    ))
                                }
                                data::config::preview::CardImageAction::Preview => {
                                    Message::ImagePreview(card.image.clone())
                                }
                            })
                            .padding(0)
                            .style(theme::button::bare),
                    );
                }

                keyed(
                    keyed::Key::Preview(message.hash, index),
                    container(card_content).padding(8).style(
                        |theme: &Theme| iced::widget::container::Style {
                            background: Some(iced::Background::Color(
                                theme.styles().buttons.secondary.background,
                            )),
                            text_color: Some(theme.styles().text.primary.color),
                            border: iced::Border {
                                radius: 4.0.into(),
                                width: 1.0,
                                color: theme.styles().general.border,
                            },
                            ..Default::default()
                        },
                    ),
                )
            }
            data::Preview::Image(img) => {
                let inner = preview_content(preview, self.config, self.theme);

                keyed(
                    keyed::Key::Preview(message.hash, index),
                    button(inner)
                        .on_press(match self.config.preview.image.action {
                            data::config::preview::ImageAction::OpenUrl => {
                                Message::Link(message::Link::Url(
                                    img.url.to_string(),
                                ))
                            }
                            data::config::preview::ImageAction::Preview => {
                                Message::ImagePreview(img.clone())
                            }
                        })
                        .padding(0)
                        .style(theme::button::bare),
                )
            }
        };

        let content = context_menu::preview(
            content,
            url.as_str(),
            self.can_send_replies,
            self.can_send_reactions,
            self.can_redact_message(message),
            message,
            selected_reactions_refs(message, self.our_nick),
            self.config,
            self.theme,
        );

        let hide_button = if is_hovered {
            container(tooltip(
                button(center(icon::cancel()))
                    .padding(5)
                    .width(HIDE_BUTTON_WIDTH)
                    .height(HIDE_BUTTON_WIDTH)
                    .on_press(Message::HidePreview(message.hash, url.clone()))
                    .style(|theme, status| {
                        theme::button::secondary(theme, status, false)
                    }),
                self.config
                    .tooltips
                    .show_for_buttons()
                    .then_some("Hide preview"),
                tooltip::Position::Top,
                self.theme,
            ))
        } else {
            container(
                space::horizontal().width(Length::Fixed(HIDE_BUTTON_WIDTH)),
            )
        };

        // Iced hack: using a stack with right-aligned hide_button ensures the button always stays visible
        // at the edge of the content, even when the parent container is resized to a smaller width.
        let stack = stack![
            container(content).padding(padding::right(HIDE_BUTTON_WIDTH + 2.0)),
            right(hide_button),
        ];

        let content = container(stack)
            .align_y(alignment::Vertical::Top)
            .width(Length::Fill)
            .padding(padding::top(4).bottom(4));

        mouse_area(content)
            .on_enter(Message::PreviewHovered(message.hash, index))
            .on_exit(Message::PreviewUnhovered(message.hash, index))
            .into()
    }

    fn format_user_message(
        &self,
        message: &'a data::Message,
        hidden_fragments: &[usize],
        right_alignment_middle_width: Option<f32>,
        user: &'a User,
        hide_nickname: bool,
        nick_prefix_to_strip: Option<&str>,
    ) -> (
        Option<Element<'a, Message>>,
        Element<'a, Message>,
        Vec<Element<'a, Message>>,
    ) {
        let not_sent_row = self.not_sent_row(message);

        let dimmed = (not_sent_row.is_some() || message.redaction.is_some())
            .then_some(Dimmed::new(None));
        let dimmed_background_tuple = dimmed
            .map(|dimmed| (dimmed, self.theme.styles().buffer.background));

        let user_in_channel =
            self.target.users().and_then(|users| users.resolve(user));
        let rerouted_private = message.is_rerouted();
        let is_user_away = match self.config.buffer.nickname.shown_status {
            ShownStatus::Current => user_in_channel.unwrap_or(user),
            ShownStatus::Historical => user,
        }
        .is_away();
        let is_user_offline = if rerouted_private {
            false
        } else {
            match self.config.buffer.nickname.shown_status {
                ShownStatus::Current => {
                    self.target.is_channel() && user_in_channel.is_none()
                }
                ShownStatus::Historical => false,
            }
        };
        let is_ourself = self
            .target
            .our_user()
            .is_some_and(|our_user| our_user.nickname() == user.nickname());

        let user_display = UserDisplay::new(
            user,
            self.config.buffer.nickname.show_access_levels,
            self.config.buffer.nickname.show_bot_icon,
            self.registry,
            &self.config.display.nickname,
            self.config.buffer.nickname.truncate,
            self.config.display.truncation_character,
            Some(&self.config.buffer.nickname.brackets),
            true,
        );

        let nick_element: Element<_> = if hide_nickname {
            let width = if let Some(right_alignment_middle_width) =
                right_alignment_middle_width
            {
                right_alignment_middle_width
            } else {
                user_display.width(self.config)
            };

            Space::new().width(width).into()
        } else {
            let mut nick_text = user_display.into_element(
                user,
                is_user_away,
                is_user_offline,
                dimmed_background_tuple,
                None,
                false,
                true,
                self.theme,
                self.config,
            );

            if let Some(width) = right_alignment_middle_width {
                nick_text = container(nick_text)
                    .width(width)
                    .align_x(text::Alignment::Right)
                    .into();
            }

            if rerouted_private && user_in_channel.is_none() {
                context_menu::rerouted_private_user(
                    nick_text,
                    self.server,
                    self.prefix,
                    self.registry,
                    self.previews.collection(),
                    user,
                    self.config,
                    self.theme,
                    &self.config.buffer.nickname.click,
                )
                .map(Message::ContextMenu)
            } else {
                context_menu::user(
                    nick_text,
                    self.server,
                    self.prefix,
                    self.target.channel(),
                    self.registry,
                    self.previews.collection(),
                    user,
                    user_in_channel,
                    self.target.our_user(),
                    self.config,
                    self.theme,
                    &self.config.buffer.nickname.click,
                )
                .map(Message::ContextMenu)
            }
        };

        let formatter = *self;

        let message_style = move |message_theme: &Theme| {
            theme::selectable_text::dimmed(
                if rerouted_private {
                    theme::selectable_text::tertiary(message_theme)
                } else {
                    theme::selectable_text::default(message_theme)
                },
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

        let redaction_message =
            message.redaction.as_ref().map(Redaction::message);

        let (message_content, after_content) =
            if self.config.buffer.redaction.display.is_redacted()
                && !message.expanded
                && let Some(redaction_message) = redaction_message
            {
                (
                    button(
                        selectable_text(redaction_message)
                            .font_maybe(
                                theme::font_style::primary(self.theme)
                                    .map(font::get),
                            )
                            .style(message_style),
                    )
                    .style(theme::button::bare)
                    .padding(0)
                    .on_press(Message::Link(message::Link::ExpandMessage(
                        message.server_time,
                        message.hash,
                    )))
                    .into(),
                    vec![],
                )
            } else {
                (
                    tooltip(
                        message_content::with_context(
                            &message.content,
                            hidden_fragments,
                            self.server,
                            self.registry,
                            self.chantypes,
                            self.casemapping,
                            self.theme,
                            Message::Link,
                            None,
                            message_style,
                            theme::font_style::primary,
                            color_transformation,
                            move |link| match link {
                                message::Link::User(_, _) => {
                                    if rerouted_private
                                        && !is_ourself
                                        && user_in_channel.is_none()
                                    {
                                        vec![context_menu::Entry::Whois]
                                    } else {
                                        context_menu::Entry::user_list(
                                            formatter.target.is_channel(),
                                            user_in_channel,
                                            formatter.target.our_user(),
                                            formatter
                                                .config
                                                .file_transfer
                                                .enabled,
                                            context_menu::has_user_metadata(
                                                user,
                                                formatter.registry,
                                                formatter.config,
                                            ),
                                        )
                                    }
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
                            nick_prefix_to_strip,
                            self.config,
                        ),
                        redaction_message,
                        tooltip::Position::Top,
                        self.theme,
                    ),
                    self.reaction_row(message)
                        .into_iter()
                        .chain(not_sent_row)
                        .collect(),
                )
            };

        (Some(nick_element), message_content, after_content)
    }

    fn format_server_message(
        &self,
        message: &'a data::Message,
        hidden_fragments: &[usize],
        right_alignment_middle_width: Option<f32>,
        server: Option<&'a message::source::Server>,
    ) -> (
        Option<Element<'a, Message>>,
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

        let link = message.expanded.then_some(message::Link::ContractMessage(
            message.server_time,
            message.hash,
        ));

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
            right_alignment_middle_width,
            self.config,
            marker_style,
            link.clone().map(Message::Link),
        );

        let message_content = message_content::with_context(
            &message.content,
            hidden_fragments,
            formatter.server,
            formatter.registry,
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
                    let user_in_channel =
                        formatter.target.users().and_then(|u| u.resolve(user));

                    context_menu::Entry::user_list(
                        formatter.target.is_channel(),
                        user_in_channel,
                        formatter.target.our_user(),
                        formatter.config.file_transfer.enabled,
                        context_menu::has_user_metadata(
                            user,
                            formatter.registry,
                            formatter.config,
                        ),
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
            None,
            self.config,
        );

        (
            Some(marker),
            message_content,
            formatter.reaction_row(message).into_iter().collect(),
        )
    }

    fn format_condensed_message(
        &self,
        message: &'a data::Message,
        right_alignment_middle_width: Option<f32>,
        hide_timestamp: bool,
    ) -> (
        Option<Element<'a, Message>>,
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

        let link =
            message::Link::ExpandMessage(message.server_time, message.hash);
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

        let middle_is_some = range_end_timestamp.is_some()
            || !matches!(condensation_marker, Marker::None)
            || right_alignment_middle_width.is_some();

        let middle = middle_is_some.then_some({
            let space = if range_end_timestamp.is_none()
                || matches!(condensation_marker, Marker::None)
            {
                None
            } else {
                Some(Element::from(selectable_text(" ")))
            };

            let marker = message_marker(
                condensation_marker,
                None,
                self.config,
                theme::selectable_text::condensed_marker,
                Some(Message::Link(link.clone())),
            );

            let middle = row![
                range_end_timestamp,
                space,
                if right_alignment_middle_width.is_some() {
                    container(marker)
                        .width(Length::Fill)
                        .align_x(text::Alignment::Right)
                        .into()
                } else {
                    marker
                }
            ];

            if let Some(right_alignment_middle_width) =
                right_alignment_middle_width
            {
                container(middle).width(right_alignment_middle_width).into()
            } else {
                middle.into()
            }
        });

        let message_content = message_content::with_context(
            &message.content,
            &[],
            formatter.server,
            formatter.registry,
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
                    let user_in_channel =
                        formatter.target.users().and_then(|u| u.resolve(user));

                    context_menu::Entry::user_list(
                        formatter.target.is_channel(),
                        user_in_channel,
                        formatter.target.our_user(),
                        formatter.config.file_transfer.enabled,
                        context_menu::has_user_metadata(
                            user,
                            formatter.registry,
                            formatter.config,
                        ),
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
            None,
            self.config,
        );

        (middle, container(message_content).into(), vec![])
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
                registry: self.registry,
                avatar: context_menu::user_avatar(
                    user,
                    self.registry,
                    self.previews.collection(),
                ),
                user,
                current_user,
            })
        } else {
            let selected_reaction_texts =
                selected_reactions_refs(message, self.our_nick);

            link.url().map(move |url| Context::Url {
                url,
                message: Some(message),
                selected_reactions: selected_reaction_texts,
            })
        }
    }
}

impl<'a> LayoutMessage<'a> for ChannelQueryLayout<'a> {
    fn should_track_reply_target_visibility(&self) -> bool {
        self.can_send_replies
            && self.config.buffer.reply.enabled
            && self.config.buffer.reply.highlight_hovered_message
    }

    fn format(
        &self,
        message: &'a data::Message,
        right_alignment_widths: Option<RightAlignmentWidths>,
        hide_timestamp: bool,
        hide_nickname: bool,
        visible_for_source: Option<
            &impl Fn(&Preview, &message::Source) -> bool,
        >,
        visible_url_messages: &HashMap<message::Hash, Vec<url::Url>>,
        hovered_preview: Option<(message::Hash, usize)>,
        hovered_reply: Option<message::Hash>,
    ) -> Option<Element<'a, Message>> {
        let mut prefixes: Option<Element<_>> = self.format_prefixes(message);

        if let Some(right_alignment_widths) = right_alignment_widths {
            prefixes = Some(prefixes.map_or(
                Space::new().width(right_alignment_widths.prefixes).into(),
                |prefixes| {
                    container(prefixes)
                        .width(right_alignment_widths.prefixes)
                        .into()
                },
            ));
        }

        let mut timestamp: Option<Element<_>> =
            self.format_timestamp(message, hide_timestamp);

        if let Some(right_alignment_widths) = right_alignment_widths {
            timestamp = Some(timestamp.map_or(
                Space::new().width(right_alignment_widths.timestamp).into(),
                |timestamp| {
                    container(timestamp)
                        .width(right_alignment_widths.timestamp)
                        .into()
                },
            ));
        }

        let left_is_hidden = prefixes.is_none() && hide_timestamp;

        let right_alignment_middle_width = right_alignment_widths
            .map(|right_alignment_widths| right_alignment_widths.middle);

        let (
            message_has_urls,
            enumerated_previews,
            hidden_fragments,
            is_visible_url_message,
        ) = if self.previews_enabled(message)
            && let message::Content::Fragments(fragments) = &message.content
        {
            let urls = eligible_preview_urls(
                fragments,
                &message.hidden_urls,
                self.config.preview.max_per_message,
            );

            if !urls.is_empty() {
                let is_visible_url_message =
                    visible_url_messages.contains_key(&message.hash);

                let enumerated_urls = urls
                    .into_iter()
                    .enumerate()
                    .map(|(url_index, (fragment_index, url))| {
                        if let Some(preview::State::Loaded(preview)) =
                            self.previews.get(url)
                            && visible_for_source.is_none_or(
                                |visible_for_source| {
                                    visible_for_source(
                                        preview,
                                        message.target.source(),
                                    )
                                },
                            )
                        {
                            (fragment_index, url_index, url, Some(preview))
                        } else {
                            (fragment_index, url_index, url, None)
                        }
                    })
                    .collect::<Vec<_>>();

                let loaded_previews: Vec<(usize, &Preview)> = enumerated_urls
                    .iter()
                    .filter_map(|(fragment_index, _, _, preview)| {
                        preview.map(|p| (*fragment_index, p))
                    })
                    .collect();

                let hidden_fragments = compute_hidden_fragments(
                    fragments,
                    &loaded_previews,
                    self.config,
                );

                (
                    true,
                    enumerated_urls
                        .into_iter()
                        .map(|(_, url_index, url, preview)| {
                            (url_index, url, preview)
                        })
                        .collect(),
                    hidden_fragments,
                    is_visible_url_message,
                )
            } else {
                (false, vec![], vec![], false)
            }
        } else {
            (false, vec![], vec![], false)
        };

        let reply_nick_to_strip = self.reply_nick_to_strip(message);

        let (middle, content, after_content): (
            Option<Element<'a, Message>>,
            Element<'a, Message>,
            Vec<Element<'a, Message>>,
        ) = match message.target.source() {
            message::Source::User(user) => Some(self.format_user_message(
                message,
                &hidden_fragments,
                right_alignment_middle_width,
                user,
                hide_nickname,
                reply_nick_to_strip,
            )),
            message::Source::Server(server_message) => {
                Some(self.format_server_message(
                    message,
                    &hidden_fragments,
                    right_alignment_middle_width,
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
                    right_alignment_middle_width,
                    self.config,
                    message_style,
                    None,
                );

                let message_content = message_content::with_context(
                    &message.content,
                    &hidden_fragments,
                    formatter.server,
                    formatter.registry,
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
                                .and_then(|u| u.resolve(user));

                            context_menu::Entry::user_list(
                                formatter.target.is_channel(),
                                user_in_channel,
                                formatter.target.our_user(),
                                formatter.config.file_transfer.enabled,
                                context_menu::has_user_metadata(
                                    user,
                                    formatter.registry,
                                    formatter.config,
                                ),
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
                    None,
                    formatter.config,
                );

                let after_content =
                    self.reaction_row(message).into_iter().chain(not_sent_row);

                Some((Some(marker), message_content, after_content.collect()))
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
                    right_alignment_middle_width,
                    self.config,
                    message_style,
                    None,
                );

                let message = message_content(
                    &message.content,
                    &[],
                    self.server,
                    self.registry,
                    self.chantypes,
                    self.casemapping,
                    self.theme,
                    Message::Link,
                    None,
                    message_style,
                    message_font_style,
                    Option::<fn(Color) -> Color>::None,
                    None,
                    self.config,
                );

                Some((Some(marker), message, vec![]))
            }
            message::Source::Internal(message::source::Internal::Logs(_)) => {
                None
            }
            message::Source::Internal(
                message::source::Internal::Condensed(_),
            ) => (!message.text().is_empty()).then_some(
                self.format_condensed_message(
                    message,
                    right_alignment_middle_width,
                    hide_timestamp,
                ),
            ),
        }?;

        let selected_reaction_texts =
            selected_reactions(message, self.our_nick);

        let content = context_menu::message(
            content,
            message,
            selected_reaction_texts,
            self.can_send_replies,
            self.can_send_reactions,
            self.can_redact_message(message),
            self.config,
            self.theme,
        );

        let middle_is_some = middle.is_some();

        let row = row![
            prefixes,
            timestamp,
            if left_is_hidden {
                let width = font::width_from_str(" ", &self.config.font);

                Element::from(Space::new().width(width))
            } else {
                Element::from(selectable_text(" "))
            },
            middle,
            middle_is_some.then_some(selectable_text(" ")),
        ];

        let content = if message_has_urls {
            let mut column = column![].spacing(2);

            let show_message_content = if hidden_fragments.is_empty() {
                true
            } else if let message::Content::Fragments(fragments) =
                &message.content
            {
                fragments.iter().enumerate().any(|(index, fragment)| {
                    !hidden_fragments.contains(&index)
                        && !fragment.as_str().trim_end().is_empty()
                })
            } else {
                true
            };

            if show_message_content {
                column = column.push(content);
            }

            for (index, url, preview) in &enumerated_previews {
                if let Some(preview) = preview {
                    let is_hovered = hovered_preview.is_some_and(
                        |(hovered_hash, hovered_index)| {
                            hovered_hash == message.hash
                                && hovered_index == *index
                        },
                    );

                    column = column.push(self.preview_row(
                        message, preview, url, *index, is_hovered,
                    ));
                }
            }

            if is_visible_url_message {
                notify_visibility(
                    column,
                    2000.0,
                    notify_visibility::When::Disjoint,
                    message.hash,
                    Message::ExitingViewport(message.hash),
                )
            } else {
                notify_visibility(
                    column,
                    1000.0,
                    notify_visibility::When::Intersecting,
                    message.hash,
                    Message::EnteringViewport(
                        message.hash,
                        enumerated_previews
                            .into_iter()
                            .map(|(_, url, _)| url)
                            .cloned()
                            .collect(),
                    ),
                )
            }
        } else {
            content
        };

        let content = if after_content.is_empty() {
            content
        } else {
            column![content].extend(after_content).into()
        };

        let message_element = if self.content_on_new_line(message) {
            container(column![row, content]).into()
        } else {
            container(row![row, content]).into()
        };

        let message_element = if let Some(reply_row) = self.reply_line(
            message,
            right_alignment_middle_width,
            hovered_reply,
        ) {
            column![reply_row, message_element].into()
        } else {
            message_element
        };

        Some(message_element)
    }
}

impl<'a> ChannelQueryLayout<'a> {
    fn target_kind(&self) -> history::Kind {
        match self.target {
            TargetInfo::Channel { channel, .. } => {
                history::Kind::Channel(self.server.clone(), (*channel).clone())
            }
            TargetInfo::Query { query } => {
                history::Kind::Query(self.server.clone(), (*query).clone())
            }
        }
    }

    fn reply_preview_urls(&self, message: &data::Message) -> Vec<url::Url> {
        if !self.config.buffer.reply.tooltip.enabled {
            return vec![];
        }
        let Some(reply_preview) = &message.reply_preview else {
            return vec![];
        };
        if reply_preview.blocked {
            return vec![];
        }
        let message::Content::Fragments(fragments) = &reply_preview.content
        else {
            return vec![];
        };
        let kind = self.target_kind();
        fragments
            .iter()
            .filter_map(message::Fragment::url)
            .filter(|url| {
                self.config.preview.is_enabled(url.as_str())
                    && !self.history.is_preview_hidden(
                        &kind,
                        reply_preview.hash,
                        reply_preview.server_time,
                        url,
                    )
            })
            .take(self.config.preview.max_per_message)
            .cloned()
            .collect()
    }

    /// Generates the reply line element to be used in buffers: `┌── ↩ alice: hi bob`
    fn reply_line(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
        hovered_reply: Option<message::Hash>,
    ) -> Option<Element<'a, Message>> {
        message.reply_to.as_ref()?;

        if !self.config.buffer.reply.enabled {
            return None;
        }

        let text_size =
            self.config.font.size.map_or(theme::TEXT_SIZE, f32::from);
        let preview_text_size = text_size * 0.85;
        let show_reply_icon = self.config.buffer.reply.show_icon;

        let mut hover_tooltip: Option<Element<'_, _>> = None;

        let preview: Element<'_, _> = if let Some(reply_preview) =
            &message.reply_preview
        {
            let data::message::ReplyPreview {
                user,
                content: reply_content,
                in_reply_to,
                redaction: reply_redaction,
                blocked: reply_blocked,
                is_action: reply_is_action,
                ..
            } = reply_preview;

            let nick_info: Option<(&User, bool, UserDisplay)> =
                if let Some(user) = user {
                    let is_our_nick = self
                        .our_nick
                        .is_some_and(|our| our == user.nickname())
                        && !(message.is_echo
                            || message.direction == message::Direction::Sent);

                    let user = self
                        .target
                        .users()
                        .and_then(|users| users.resolve(user))
                        .unwrap_or(user);

                    let highlight = is_our_nick
                        && self.config.highlights.nickname.is_target_included(
                            message.user(),
                            self.target.as_target_ref(),
                            self.server,
                            self.casemapping,
                        );

                    let display = UserDisplay::new(
                        user,
                        self.config.buffer.nickname.show_access_levels,
                        self.config.buffer.nickname.show_bot_icon,
                        self.registry,
                        &self.config.display.nickname,
                        self.config.buffer.nickname.truncate,
                        self.config.display.truncation_character,
                        Some(&self.config.buffer.nickname.brackets),
                        false,
                    );

                    Some((user, highlight, display))
                } else {
                    None
                };

            hover_tooltip = (self.config.buffer.reply.tooltip.enabled
                && !reply_blocked
                && hovered_reply != Some(reply_preview.hash))
            .then(|| {
                let tooltip_nick = (!reply_is_action)
                    .then(|| {
                        nick_info.as_ref().map(|(user, _, display)| {
                            display.clone().into_element(
                                user,
                                false,
                                false,
                                None,
                                None,
                                false,
                                false,
                                self.theme,
                                self.config,
                            )
                        })
                    })
                    .flatten();
                self.reply_hover_tooltip(
                    reply_preview.hash,
                    reply_preview.server_time,
                    tooltip_nick,
                    reply_content,
                    in_reply_to.as_deref(),
                    reply_redaction.as_ref(),
                    *reply_blocked,
                    *reply_is_action,
                )
            });

            let highlight = nick_info.as_ref().is_some_and(|(_, h, _)| *h);
            reply_preview_content(
                Some(reply_preview),
                highlight,
                show_reply_icon,
                preview_text_size,
                self.target.users(),
                self.registry,
                self.config,
                self.theme,
            )
        } else {
            reply_preview_content(
                None,
                false,
                show_reply_icon,
                preview_text_size,
                self.target.users(),
                self.registry,
                self.config,
                self.theme,
            )
        };

        let char_width = font::width_from_str("a", &self.config.font);

        let timestamp_chars = self
            .config
            .buffer
            .format_timestamp(&message.server_time)
            .map_or(0, |s| s.chars().count());

        // right-aligned: fixed short arm offset to content column.
        // left-aligned / top-aligned: arm spans from timestamp midpoint to its edge.
        let arm_text: Element<'_, _> = if let Some(nick_col_width) =
            right_aligned_width
        {
            let nick_col_chars = (nick_col_width / char_width).round() as usize;
            let indent = timestamp_chars + nick_col_chars - 2;
            text(" ".repeat(indent) + "┌──")
                .size(text_size)
                .style(theme::text::timestamp)
                .line_height(LineHeight::Relative(1.0))
                .into()
        } else {
            let half = timestamp_chars / 2;
            let arm = format!(
                "{}┌{}",
                " ".repeat(half),
                "─".repeat(
                    half.saturating_sub(1)
                        + usize::from(!timestamp_chars.is_multiple_of(2))
                ),
            );
            text(arm)
                .size(text_size)
                .style(theme::text::timestamp)
                .line_height(LineHeight::Relative(1.0))
                .into()
        };

        let delay = iced::time::Duration::from_millis(
            self.config.buffer.reply.tooltip.delay,
        );
        let reply_urls = self.reply_preview_urls(message);

        let interactive: Element<'_, _> =
            if let (Some(reply_preview), Some(channel)) =
                (&message.reply_preview, self.target.channel())
            {
                let server = self.server.clone();
                let channel = channel.clone();
                let hash = reply_preview.hash;
                button(preview)
                    .style(theme::button::reply_preview)
                    .padding(0)
                    .on_press(Message::Link(message::Link::GoToMessage(
                        server, channel, hash,
                    )))
                    .into()
            } else {
                // not interactive — use a container to preserve muted colors
                container(preview)
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.styles().text.secondary.color),
                        ..Default::default()
                    })
                    .into()
            };

        let interactive: Element<'_, _> = if let Some(tooltip) = hover_tooltip {
            iced::widget::tooltip(
                interactive,
                container(tooltip).padding(
                    iced::Padding::new(0.0).bottom(2.0).top(2.0).right(2.0),
                ),
                tooltip::Position::TopLeft,
            )
            .smart_placement(true)
            .padding(0) // this only takes uniform padding; we wrap in a container above to get what we want
            .delay(delay)
            .into()
        } else {
            interactive
        };

        let interactive: Element<'_, _> =
            if let Some(reply_preview) = &message.reply_preview {
                mouse_area(interactive)
                    .on_enter(Message::ReplyPreviewHovered(
                        message.hash,
                        reply_preview.hash,
                        reply_urls,
                    ))
                    .on_exit(Message::ReplyPreviewUnhovered(message.hash))
                    .into()
            } else {
                interactive
            };

        let element: Element<'_, _> =
            row![arm_text, interactive].spacing(char_width).into();

        Some(element)
    }

    /// Generates the hover preview for a reply
    fn reply_hover_tooltip(
        &self,
        reply_hash: message::Hash,
        server_time: chrono::DateTime<chrono::Utc>,
        nick: Option<Element<'a, Message>>,
        reply: &'a message::Content,
        in_reply_to: Option<&'a message::ReplyPreview>,
        redaction: Option<&'a data::redaction::Redaction>,
        is_blocked: bool,
        is_action: bool,
    ) -> Element<'a, Message> {
        let kind = self.target_kind();
        let text_size =
            self.config.font.size.map_or(theme::TEXT_SIZE, f32::from);
        let preview_text_size = text_size * 0.85;

        let is_muted = redaction.is_some() || is_blocked;
        let dimmed = is_muted.then_some(Dimmed::new(None));
        let bg = self.theme.styles().buffer.background;
        let dimmed_bg = dimmed.map(|d| (d, bg));
        let dimmed_style = move |t: &Theme| {
            theme::selectable_text::dimmed(
                theme::selectable_text::default(t),
                t,
                dimmed_bg,
            )
        };

        let use_dimmed_display = redaction.is_some()
            && self.config.buffer.redaction.display
                == data::config::buffer::redaction::Display::Dimmed;

        let (loaded, hidden_fragments) =
            if let message::Content::Fragments(fragments) = reply {
                let loaded: Vec<(usize, &Preview)> = fragments
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, f)| f.url().map(|url| (idx, url)))
                    .filter(|(_, url)| {
                        self.config.preview.is_enabled(url.as_str())
                            && !self.history.is_preview_hidden(
                                &kind,
                                reply_hash,
                                server_time,
                                url,
                            )
                    })
                    .take(self.config.preview.max_per_message)
                    .filter_map(|(fragment_idx, url)| {
                        match self.previews.get(url) {
                            Some(preview::State::Loaded(p)) => {
                                Some((fragment_idx, p as &Preview))
                            }
                            _ => None,
                        }
                    })
                    .collect();

                let hidden =
                    compute_hidden_fragments(fragments, &loaded, self.config);

                (loaded, hidden)
            } else {
                (vec![], vec![])
            };

        let (tooltip_content, show_previews): (Element<_>, bool) = if let Some(
            redaction,
        ) =
            redaction.filter(|_| !use_dimmed_display)
        {
            (
                selectable_text(redaction.message())
                    .style(dimmed_style)
                    .into(),
                false,
            )
        } else {
            let content_style = move |t: &Theme| {
                theme::selectable_text::dimmed(
                    if is_action {
                        theme::selectable_text::action(t)
                    } else {
                        theme::selectable_text::default(t)
                    },
                    t,
                    dimmed_bg,
                )
            };
            let tooltip_nick =
                self.config.buffer.reply.hide_redundant_nicks.then(|| {
                    in_reply_to
                        .and_then(|p| p.user.as_ref())
                        .map(|u| u.nickname().as_str().to_owned())
                });
            let tooltip_nick = tooltip_nick.flatten();
            let max_chars = self.config.buffer.reply.tooltip.max_chars;
            let preview = strip_leading_nick_from_preview(
                reply.preview_text(),
                tooltip_nick.as_deref(),
            );
            let truncated = (max_chars > 0
                && preview.chars().count() > max_chars)
                .then(|| {
                    preview
                        .chars()
                        .take(max_chars)
                        .chain(std::iter::once('…'))
                        .collect::<String>()
                });

            if let Some(truncated) = truncated {
                (
                    selectable_text(truncated).style(content_style).into(),
                    false,
                )
            } else {
                (
                    message_content(
                        reply,
                        &hidden_fragments,
                        self.server,
                        self.registry,
                        self.chantypes,
                        self.casemapping,
                        self.theme,
                        Message::Link,
                        None,
                        content_style,
                        if is_action {
                            theme::font_style::action
                        } else {
                            theme::font_style::primary
                        },
                        dimmed.map(|d| {
                            move |color: Color| d.transform_color(color, bg)
                        }),
                        tooltip_nick.as_deref(),
                        self.config,
                    ),
                    true,
                )
            }
        };

        let show_message_content = if hidden_fragments.is_empty() {
            true
        } else if let message::Content::Fragments(fragments) = reply {
            fragments.iter().enumerate().any(|(index, fragment)| {
                !hidden_fragments.contains(&index)
                    && !fragment.as_str().trim_end().is_empty()
            })
        } else {
            true
        };

        let mut content_col = column![].spacing(4);
        if show_message_content {
            content_col = content_col.push(tooltip_content);
        }

        if show_previews {
            for (_, preview) in &loaded {
                let el = preview_content(preview, self.config, self.theme);
                let el: Element<_> = match preview {
                    data::Preview::Card(..) => button(el)
                        .style(|theme, _| {
                            theme::button::preview_card(
                                theme,
                                // force active state so we don't get the fallback disabled state
                                // which is unstyled
                                iced::widget::button::Status::Active,
                            )
                        })
                        .into(),
                    data::Preview::Image(..) => {
                        container(el).max_height(200).into()
                    }
                };
                content_col = content_col.push(el);
            }
        }

        let in_reply_to_row: Option<Element<_>> = in_reply_to.map(|nested| {
            reply_preview_content(
                Some(nested),
                false,
                true,
                preview_text_size,
                self.target.users(),
                self.registry,
                self.config,
                self.theme,
            )
        });

        let action_marker: Option<Element<_>> = is_action.then(|| {
            message_marker(
                Marker::Dot,
                None,
                self.config,
                theme::selectable_text::action,
                None::<Message>,
            )
        });

        let body: Element<_> = row![]
            .spacing(font::width_from_str(" ", &self.config.font))
            .extend(self.config.buffer.format_timestamp(&server_time).map(
                |timestamp| -> Element<_> {
                    text(timestamp).style(theme::text::timestamp).into()
                },
            ))
            .extend(action_marker)
            .extend(nick)
            .push(content_col)
            .into();

        let outer_col = column![].spacing(4).extend(in_reply_to_row).push(body);

        container(outer_col)
            .style(theme::container::hover_preview_tooltip)
            .padding(8)
            .max_width(self.config.buffer.reply.tooltip.max_width)
            .into()
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

fn eligible_preview_urls<'a>(
    fragments: &'a [message::Fragment],
    hidden_urls: &HashSet<url::Url>,
    max_per_message: usize,
) -> Vec<(usize, &'a url::Url)> {
    fragments
        .iter()
        .enumerate()
        .filter_map(|(index, fragment)| fragment.url().map(|url| (index, url)))
        .filter(|(_, url)| !hidden_urls.contains(*url))
        .take(max_per_message)
        .collect()
}

// Determines which fragment indices should have their URL text hidden
fn compute_hidden_fragments(
    fragments: &[message::Fragment],
    loaded_previews: &[(usize, &Preview)],
    config: &data::Config,
) -> Vec<usize> {
    let mut hidden = vec![];
    let mut is_trailing = true;

    for (fragment_index, preview) in loaded_previews.iter().rev() {
        if is_trailing {
            let trailing: Vec<_> = fragments
                .iter()
                .enumerate()
                .skip(fragment_index.saturating_add(1))
                .collect();
            is_trailing = trailing.is_empty()
                || trailing.iter().all(|(index, fragment)| {
                    hidden.contains(index)
                        || fragment.as_str().trim_end().is_empty()
                });
        }

        match config.preview.hide_url_when(preview) {
            HideUrlCondition::ContainsOnlyUrl => {
                if fragments.len() == 1 {
                    hidden.push(*fragment_index);
                }
            }
            HideUrlCondition::Trailing => {
                if is_trailing {
                    hidden.push(*fragment_index);
                }
            }
            HideUrlCondition::Never => (),
        }
    }

    hidden
}

fn strip_leading_nick_from_preview(text: String, nick: Option<&str>) -> String {
    let Some(nick) = nick else { return text };
    message_content::strip_leading_nick(&text, nick)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or(text)
}
