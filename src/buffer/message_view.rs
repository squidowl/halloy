use chrono::{TimeDelta, Utc};
use data::config::buffer::Dimmed;
use data::config::buffer::nickname::ShownStatus;
use data::isupport::{CaseMap, PrefixMap};
use data::server::Server;
use data::user::ChannelUsers;
use data::{Config, User, message, target};
use iced::widget::text::LineHeight;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Color, Length, alignment};

use super::context_menu::{self, Context};
use super::scroll_view::LayoutMessage;
use crate::buffer::scroll_view::Message;
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
    pub supports_echoes: bool,
    pub connected: bool,
    pub server: &'a Server,
    pub theme: &'a Theme,
    pub target: TargetInfo<'a>,
}

impl<'a> ChannelQueryLayout<'a> {
    fn format_timestamp(
        &self,
        message: &'a data::Message,
    ) -> Option<Element<'a, Message>> {
        if let message::Source::Internal(message::source::Internal::Condensed(
            end_server_time,
        )) = message.target.source()
            && message.server_time != *end_server_time
        {
            self.config
                .buffer
                .format_range_timestamp(&message.server_time, end_server_time)
                .map(|(start_timestamp, dash, end_timestamp)| {
                    row![
                        context_menu::timestamp(
                            selectable_text(start_timestamp)
                                .style(theme::selectable_text::timestamp)
                                .font_maybe(
                                    theme::font_style::timestamp(self.theme)
                                        .map(font::get),
                                ),
                            &message.server_time,
                            self.config,
                            self.theme,
                        )
                        .map(Message::ContextMenu),
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
                        .map(Message::ContextMenu)
                    ]
                    .into()
                })
        } else {
            self.config
                .buffer
                .format_timestamp(&message.server_time)
                .map(|timestamp| {
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
    }

    fn format_prefixes(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
        max_prefix_width: Option<f32>,
    ) -> Option<Element<'a, Message>> {
        message
            .target
            .prefixes()
            .map_or(
                right_aligned_width.and_then(|_| {
                    max_prefix_width.map(|width| {
                        selectable_text("")
                            .width(width)
                            .align_x(text::Alignment::Right)
                    })
                }),
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
                        Some(text.width(width).align_x(text::Alignment::Right))
                    } else {
                        Some(text)
                    }
                },
            )
            .map(Element::from)
    }

    fn format_user_message(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
        user: &'a User,
        hide_nickname: bool,
    ) -> (Element<'a, Message>, Element<'a, Message>) {
        let not_sent = (self.supports_echoes || message.command.is_some())
            && matches!(message.direction, message::Direction::Sent)
            && Utc::now().signed_duration_since(message.server_time)
                > TimeDelta::seconds(10);

        let dimmed = not_sent.then_some(Dimmed::new(None));
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

        let mut nick_text = selectable_text(
            self.config
                .buffer
                .nickname
                .brackets
                .format(user.display(with_access_levels, truncate)),
        )
        .style(move |_| nickname_style)
        .font_maybe(
            theme::font_style::nickname(self.theme, is_user_offline)
                .map(font::get),
        );

        if let Some(width) = right_aligned_width {
            nick_text = nick_text.width(width).align_x(text::Alignment::Right);
        }

        let nick = tooltip(
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
            // We show the full nickname in the tooltip if truncation is enabled.
            truncate.map(|_| user.as_str()),
            tooltip::Position::Bottom,
            self.theme,
        );

        let nick_element: Element<_> = if hide_nickname {
            let width = match self.config.buffer.nickname.alignment {
                data::buffer::Alignment::Left
                | data::buffer::Alignment::Top => 0.0,
                data::buffer::Alignment::Right => {
                    right_aligned_width.unwrap_or_default()
                }
            };
            Space::new().width(width).into()
        } else {
            nick
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
                message::Link::Url(_) => context_menu::Entry::url_list(),
                _ => vec![],
            },
            move |link, entry, length| {
                entry
                    .view(
                        formatter.link_context(link),
                        length,
                        formatter.config,
                        formatter.theme,
                    )
                    .map(Message::ContextMenu)
            },
            self.config,
        );

        let content = if not_sent {
            let font_size = 0.85
                * self.config.font.size.map_or(theme::TEXT_SIZE, f32::from);
            let icon_size =
                LineHeight::default().to_absolute(font_size.into()).0;

            Element::from(column![
                message_content,
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
                        .align_y(alignment::Vertical::Center)
                    )
                    .style(|theme, status| {
                        theme::button::bare(theme, status)
                    })
                    .padding(0),
                    &message.server_time,
                    &message.hash,
                    message.command.is_some() && self.connected,
                    self.config,
                    self.theme,
                )
                .map(Message::ContextMenu)
            ])
        } else {
            Element::from(message_content)
        };

        (nick_element, content)
    }

    fn format_server_message(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
        server: Option<&'a message::source::Server>,
    ) -> (Element<'a, Message>, Element<'a, Message>) {
        let formatter = *self;

        let dimmed = formatter.config.buffer.server_messages.dimmed(server);

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
            if message.expanded {
                if message.condensed.is_some() {
                    Marker::Contract
                } else {
                    Marker::None
                }
            } else {
                Marker::Dot
            },
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
                message::Link::Url(_) => context_menu::Entry::url_list(),
                _ => vec![],
            },
            move |link, entry, length| {
                entry
                    .view(
                        formatter.link_context(link),
                        length,
                        formatter.config,
                        formatter.theme,
                    )
                    .map(Message::ContextMenu)
            },
            self.config,
        );

        (marker, container(message_content).into())
    }

    fn format_condensed_message(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
    ) -> (Element<'a, Message>, Element<'a, Message>) {
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

        let marker = message_marker(
            Marker::Expand,
            right_aligned_width,
            self.config,
            theme::selectable_text::condensed_marker,
            Some(Message::Link(link.clone())),
        );

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
                message::Link::Url(_) => context_menu::Entry::url_list(),
                _ => vec![],
            },
            move |link, entry, length| {
                entry
                    .view(
                        formatter.link_context(link),
                        length,
                        formatter.config,
                        formatter.theme,
                    )
                    .map(Message::ContextMenu)
            },
            self.config,
        );

        (marker, container(message_content).into())
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
            link.url().map(Context::Url)
        }
    }
}

impl<'a> LayoutMessage<'a> for ChannelQueryLayout<'a> {
    fn format(
        &self,
        message: &'a data::Message,
        right_aligned_width: Option<f32>,
        max_prefix_width: Option<f32>,
        range_timestamp_excess_width: Option<f32>,
        hide_nickname: bool,
    ) -> Option<Element<'a, Message>> {
        let timestamp = self.format_timestamp(message);
        let prefixes = self.format_prefixes(
            message,
            right_aligned_width,
            max_prefix_width,
        );

        let row = row![timestamp, selectable_text(" "), prefixes];

        let (middle, content): (Element<'a, Message>, Element<'a, Message>) =
            match message.target.source() {
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
                    let marker = message_marker(
                        Marker::Dot,
                        right_aligned_width,
                        self.config,
                        theme::selectable_text::action,
                        None,
                    );

                    let formatter = *self;
                    let message_content = message_content::with_context(
                        &message.content,
                        formatter.server,
                        formatter.chantypes,
                        formatter.casemapping,
                        formatter.theme,
                        Message::Link,
                        None,
                        theme::selectable_text::action,
                        theme::font_style::action,
                        Option::<fn(Color) -> Color>::None,
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
                                context_menu::Entry::url_list()
                            }
                            _ => vec![],
                        },
                        move |link, entry, length| {
                            entry
                                .view(
                                    formatter.link_context(link),
                                    length,
                                    formatter.config,
                                    formatter.theme,
                                )
                                .map(Message::ContextMenu)
                        },
                        formatter.config,
                    );

                    Some((marker, container(message_content).into()))
                }
                message::Source::Internal(
                    message::source::Internal::Status(status),
                ) => {
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

                    Some((marker, message))
                }
                message::Source::Internal(message::source::Internal::Logs(
                    _,
                )) => None,
                message::Source::Internal(
                    message::source::Internal::Condensed(end_server_time),
                ) => {
                    let right_aligned_width =
                        if message.server_time != *end_server_time {
                            right_aligned_width.map(|right_aligned_width| {
                                right_aligned_width
                                    - range_timestamp_excess_width
                                        .unwrap_or_default()
                            })
                        } else {
                            right_aligned_width
                        };

                    (!message.text().is_empty()).then_some(
                        self.format_condensed_message(
                            message,
                            right_aligned_width,
                        ),
                    )
                }
            }?;

        // When hiding consecutive nicknames (left-aligned only), insert a single space
        // to maintain visual separation. Right-aligned nicknames always get a space since
        // they don't create a gap when hidden (alignment pushes content to the same position).
        let maybe_space = if !hide_nickname
            || self.config.buffer.nickname.alignment.is_right()
        {
            Some(selectable_text(" "))
        } else {
            None
        };
        let row = row.push(middle).push(maybe_space);

        if self.content_on_new_line(message) {
            Some(container(column![row, content]).into())
        } else {
            Some(container(row![row, content]).into())
        }
    }
}
