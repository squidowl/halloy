use crate::buffer::scroll_view::Message;
use crate::widget::reaction_row::reaction_row;
use data::isupport::CaseMap;
use data::message::Reaction;
use data::server::Server;
use data::target::{self};
use data::{Config, User, message};
use iced::widget::{column, container, row, text};

use super::scroll_view::LayoutMessage;
use super::user_context;
use crate::widget::{
    Element, message_content, message_marker, selectable_text,
};
use crate::{Theme, theme};

#[derive(Clone, Copy)]
pub enum TargetInfo<'a> {
    Channel {
        channel: &'a target::Channel,
        our_user: Option<&'a User>,
        users: &'a [User],
    },
    Query,
}

impl<'a> TargetInfo<'a> {
    fn users(&self) -> &'a [User] {
        match self {
            TargetInfo::Channel { users, .. } => users,
            TargetInfo::Query => &[],
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
    pub casemapping: CaseMap,
    pub server: &'a Server,
    pub theme: &'a Theme,
    pub target: TargetInfo<'a>,
}

impl<'a> ChannelQueryLayout<'a> {
    fn format_timestamp(
        &self,
        message: &'a data::Message,
    ) -> Option<Element<'a, Message>> {
        self.config
            .buffer
            .format_timestamp(&message.server_time)
            .map(|timestamp| {
                selectable_text(timestamp)
                    .style(theme::selectable_text::timestamp)
            })
            .map(Element::from)
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
                    .style(theme::selectable_text::tertiary);

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
        let fm = *self;
        let with_access_levels = self.config.buffer.nickname.show_access_levels;
        let current_user: Option<&User> = self
            .target
            .users()
            .iter()
            .find(|current_user| *current_user == user);

        let mut text = selectable_text(
            self.config
                .buffer
                .nickname
                .brackets
                .format(user.display(with_access_levels)),
        )
        .style(|theme| {
            theme::selectable_text::nickname(theme, self.config, user)
        });

        if let Some(width) = max_nick_width {
            text = text.width(width).align_x(text::Alignment::Right);
        }

        let nick = user_context::view(
            text,
            self.server,
            self.casemapping,
            self.target.channel(),
            user,
            current_user,
            self.target.our_user(),
            self.config,
            &self.config.buffer.nickname.click,
        )
        .map(Message::UserContext);

        let message_content = message_content::with_context(
            &message.content,
            self.casemapping,
            self.theme,
            Message::Link,
            theme::selectable_text::default,
            move |link| match link {
                message::Link::User(_) => user_context::Entry::list(
                    fm.target.is_channel(),
                    fm.target.our_user(),
                ),
                _ => vec![],
            },
            move |link, entry, length| match link {
                message::Link::User(user) => entry
                    .view(
                        fm.server,
                        fm.casemapping,
                        fm.target.channel(),
                        user,
                        current_user,
                        length,
                        fm.config,
                    )
                    .map(Message::UserContext),
                _ => row![].into(),
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
        let message_style = move |message_theme: &Theme| {
            theme::selectable_text::server(message_theme, server)
        };
        let marker = message_marker(max_nick_width, message_style);
        let fm = *self;
        let message_content = message_content::with_context(
            &message.content,
            fm.casemapping,
            self.theme,
            Message::Link,
            message_style,
            move |link| match link {
                message::Link::User(_) => user_context::Entry::list(
                    fm.target.is_channel(),
                    fm.target.our_user(),
                ),
                _ => vec![],
            },
            move |link, entry, length| match link {
                message::Link::User(user) => entry
                    .view(
                        fm.server,
                        fm.casemapping,
                        fm.target.channel(),
                        user,
                        fm.target
                            .users()
                            .iter()
                            .find(|current_user| *current_user == user),
                        length,
                        fm.config,
                    )
                    .map(Message::UserContext),
                _ => row![].into(),
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
}
impl<'a> LayoutMessage<'a> for ChannelQueryLayout<'a> {
    fn format(
        &self,
        message: &'a data::Message,
        reactions: &'a [Reaction],
        max_nick_width: Option<f32>,
        max_prefix_width: Option<f32>,
    ) -> Option<Element<'a, Message>> {
        let timestamp = self.format_timestamp(message);
        let prefixes =
            self.format_prefixes(message, max_nick_width, max_prefix_width);

        let space = selectable_text(" ");

        let (middle, mut content): (
            Element<'a, Message>,
            Element<'a, Message>,
        ) = match message.target.source() {
            message::Source::User(user) => {
                Some(self.format_user_message(message, max_nick_width, user))
            }
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
                    theme::selectable_text::action,
                );

                let message_content = message_content(
                    &message.content,
                    self.casemapping,
                    self.theme,
                    Message::Link,
                    theme::selectable_text::action,
                    self.config,
                );

                let text_container = container(message_content);

                Some((marker, text_container.into()))
            }
            message::Source::Internal(message::source::Internal::Status(
                status,
            )) => {
                let message_style = move |message_theme: &Theme| {
                    theme::selectable_text::status(message_theme, *status)
                };

                let marker = message_marker(max_nick_width, message_style);

                let message = message_content(
                    &message.content,
                    self.casemapping,
                    self.theme,
                    Message::Link,
                    message_style,
                    self.config,
                );

                Some((marker, message))
            }
            message::Source::Internal(message::source::Internal::Logs) => None,
        }?;

        let initial = row![].push_maybe(timestamp).push_maybe(prefixes);

        let nick = row![middle, space];
        content = if self.content_on_new_line(message) {
            column![nick, content].into()
        } else {
            row![nick, content].into()
        };
        if !reactions.is_empty() {
            let reactions = reaction_row(
                message,
                self.target.our_user().map(|user| user.nickname()),
                reactions,
                |text| Message::Reacted {
                    // SAFETY(pounce): our message must have an ID as it has reactions which we are
                    // tracking
                    msgid: message.id.as_ref().unwrap().clone(),
                    text: text.to_owned(),
                    unreacted: false,
                },
                |text| Message::Reacted {
                    msgid: message.id.as_ref().unwrap().clone(),
                    text: text.to_owned(),
                    unreacted: true,
                },
            );
            content = column![content, reactions].into();
        }
        content = row![initial, content].into();

        Some(container(content).into())
    }
}
