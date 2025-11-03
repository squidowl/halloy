use data::config::buffer::nickname::ShownStatus;
use data::isupport::{CaseMap, PrefixMap};
use data::server::Server;
use data::user::ChannelUsers;
use data::{Config, User, message, target};
use iced::Color;
use iced::advanced::text;
use iced::widget::{column, container, row};

use super::context_menu::{self, Context};
use super::scroll_view::LayoutMessage;
use crate::buffer::scroll_view::Message;
use crate::widget::{
    Element, message_content, message_marker, selectable_text, tooltip,
};
use crate::{Theme, font, theme};

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
        max_nick_width: Option<f32>,
        max_prefix_width: Option<f32>,
    ) -> Option<Element<'a, Message>> {
        message
            .target
            .prefixes()
            .map_or(
                max_nick_width.and_then(|_| {
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
        max_nick_width: Option<f32>,
        user: &'a User,
    ) -> (Element<'a, Message>, Element<'a, Message>) {
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

        let nickname_style = theme::selectable_text::nickname(
            self.theme,
            self.config,
            match self.config.buffer.nickname.shown_status {
                ShownStatus::Current => user_in_channel.unwrap_or(user),
                ShownStatus::Historical => user,
            },
            is_user_offline,
        );

        let mut text = selectable_text(
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

        if let Some(width) = max_nick_width {
            text = text.width(width).align_x(text::Alignment::Right);
        }

        let nick = tooltip(
            context_menu::user(
                text,
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

        let formatter = *self;

        let message_content = message_content::with_context(
            &message.content,
            self.chantypes,
            self.casemapping,
            self.theme,
            Message::Link,
            theme::selectable_text::default,
            theme::font_style::primary,
            Option::<fn(Color) -> Color>::None,
            move |link| match link {
                message::Link::User(_) => context_menu::Entry::user_list(
                    formatter.target.is_channel(),
                    formatter.target.our_user(),
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

        (nick, Element::from(container(message_content)))
    }

    fn format_server_message(
        &self,
        message: &'a data::Message,
        max_nick_width: Option<f32>,
        server: Option<&'a message::source::Server>,
    ) -> (Element<'a, Message>, Element<'a, Message>) {
        let formatter = *self;

        let dimmed = formatter.config.buffer.server_messages.dimmed(server);

        let message_style = move |message_theme: &Theme| {
            let mut style =
                theme::selectable_text::server(message_theme, server);

            if let Some(dimmed) = dimmed {
                style.color = style.color.map(|color| {
                    dimmed.transform_color(
                        color,
                        formatter.theme.styles().buffer.background,
                    )
                });
            }

            style
        };
        let message_font_style = move |message_theme: &Theme| {
            theme::font_style::server(message_theme, server)
        };

        let marker = message_marker(max_nick_width, self.config, message_style);

        let message_content = message_content::with_context(
            &message.content,
            formatter.chantypes,
            formatter.casemapping,
            self.theme,
            Message::Link,
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
                message::Link::User(_) => context_menu::Entry::user_list(
                    formatter.target.is_channel(),
                    formatter.target.our_user(),
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

        (marker, container(message_content).into())
    }

    fn format_condensed_message(
        &self,
        message: &'a data::Message,
        spacer_width: Option<f32>,
    ) -> (Element<'a, Message>, Element<'a, Message>) {
        let spacer = spacer_width.map(|width| selectable_text("").width(width));

        let message_style = move |message_theme: &Theme| {
            theme::selectable_text::server(message_theme, None)
        };
        let message_font_style = move |message_theme: &Theme| {
            theme::font_style::server(message_theme, None)
        };

        let formatter = *self;
        let message_content = message_content::with_context(
            &message.content,
            formatter.chantypes,
            formatter.casemapping,
            self.theme,
            Message::Link,
            message_style,
            message_font_style,
            Some(|color: Color| -> Color {
                if let Some(dimmed) =
                    formatter.config.buffer.server_messages.condense.dimmed
                {
                    dimmed.transform_color(
                        color,
                        formatter.theme.styles().buffer.background,
                    )
                } else {
                    color
                }
            }),
            move |link| match link {
                message::Link::User(_) => context_menu::Entry::user_list(
                    formatter.target.is_channel(),
                    formatter.target.our_user(),
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

        (spacer.into(), container(message_content).into())
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
        max_nick_width: Option<f32>,
        max_prefix_width: Option<f32>,
        max_excess_timestamp_width: Option<f32>,
    ) -> Option<Element<'a, Message>> {
        let timestamp = self.format_timestamp(message);
        let prefixes =
            self.format_prefixes(message, max_nick_width, max_prefix_width);

        let row = row![timestamp, selectable_text(" "), prefixes];

        let (middle, content): (Element<'a, Message>, Element<'a, Message>) =
            match message.target.source() {
                message::Source::User(user) => Some(self.format_user_message(
                    message,
                    max_nick_width,
                    user,
                )),
                message::Source::Server(server_message) => {
                    Some(self.format_server_message(
                        message,
                        max_nick_width,
                        server_message.as_ref(),
                    ))
                }
                message::Source::Action(_) => {
                    let marker = message_marker(
                        max_nick_width,
                        self.config,
                        theme::selectable_text::action,
                    );

                    let message_content = message_content(
                        &message.content,
                        self.chantypes,
                        self.casemapping,
                        self.theme,
                        Message::Link,
                        theme::selectable_text::action,
                        theme::font_style::action,
                        Option::<fn(Color) -> Color>::None,
                        self.config,
                    );

                    let text_container = container(message_content);

                    Some((marker, text_container.into()))
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
                        max_nick_width,
                        self.config,
                        message_style,
                    );

                    let message = message_content(
                        &message.content,
                        self.chantypes,
                        self.casemapping,
                        self.theme,
                        Message::Link,
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
                    let spacer_width = if message.server_time
                        != *end_server_time
                    {
                        max_nick_width.map(|max_nick_width| {
                            max_nick_width
                                - max_excess_timestamp_width.unwrap_or_default()
                        })
                    } else {
                        max_nick_width
                    };

                    (!message.text().is_empty()).then_some(
                        self.format_condensed_message(message, spacer_width),
                    )
                }
            }?;
        let row = row.push(middle).push(selectable_text(" "));
        if self.content_on_new_line(message) {
            Some(container(column![row, content]).into())
        } else {
            Some(container(row![row, content]).into())
        }
    }
}
