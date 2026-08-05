use chrono::{DateTime, Utc};
use data::user::{Nick, NickRef, User};
use data::{Config, buffer, client, history, message, preview, target};
use iced::widget::text_editor;
use iced::{Task, widget};

use super::{context_menu, input_view, message_view, scroll_view};

#[derive(Debug, Clone)]
pub struct Manager {
    focus_capture_id: widget::Id,
    focus_capture_content: text_editor::Content,
    focused_message: Option<FocusedMessage>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            focus_capture_id: widget::Id::unique(),
            focus_capture_content: text_editor::Content::new(),
            focused_message: None,
        }
    }

    pub fn focused(&self) -> Option<FocusedMessage> {
        self.focused_message
    }

    pub fn focused_mut(&mut self) -> &mut Option<FocusedMessage> {
        &mut self.focused_message
    }

    pub fn is_focused(&self) -> bool {
        self.focused_message.is_some()
    }

    pub fn has_focused_component(&self) -> bool {
        self.focused_message
            .as_ref()
            .is_some_and(FocusedMessage::has_focused_component)
    }

    pub fn clear(&mut self) {
        self.focused_message = None;
    }

    // Returns a zero-size hidden text_editor. While a message is focused this widget
    // holds keyboard focus so that Alt+Arrow events reach the global subscription
    // rather than being consumed by the input_view text editor.
    pub fn focus_capture<'a, M: Clone + 'a>(
        &'a self,
    ) -> crate::widget::Element<'a, M> {
        widget::container(
            widget::text_editor(&self.focus_capture_content)
                .id(self.focus_capture_id.clone())
                .padding(0),
        )
        .width(0)
        .height(0)
        .into()
    }

    pub fn handle_input_event(
        &mut self,
        event: input_view::Event,
        scroll_view: &mut scroll_view::State,
        input_view: &mut input_view::State,
        upstream: &buffer::Upstream,
        kind: scroll_view::Kind<'_>,
        clients: &mut client::Map,
        history: &mut history::Manager,
        previews: &preview::Collection,
        config: &Config,
        channels_context: &dyn context_menu::ChannelsContext,
    ) -> (
        Task<scroll_view::Message>,
        Task<input_view::Message>,
        Option<context_menu::Event>,
    ) {
        match event {
            input_view::Event::NavigateFocus(direction) => {
                let was_in_mode = self.focused_message.is_some();
                let (scroll_task, scroll_event) = scroll_view.update(
                    scroll_view::Message::NavigateFocus(direction),
                    &mut self.focused_message,
                    config.buffer.chathistory.infinite_scroll,
                    kind,
                    Some(upstream),
                    history,
                    clients,
                    previews,
                    config,
                    channels_context,
                );
                if let Some(scroll_view::Event::ExitFocus) = scroll_event {
                    let (exit_task, _) = input_view.update(
                        input_view::Message::ExitFocus,
                        false,
                        upstream,
                        clients,
                        history,
                        config,
                    );
                    let refocus = input_view.focus();
                    return (
                        scroll_task,
                        Task::batch([exit_task, refocus]),
                        None,
                    );
                }
                let entered_mode =
                    !was_in_mode && self.focused_message.is_some();
                let focus_task: Task<input_view::Message> = if entered_mode {
                    widget::operation::focus(self.focus_capture_id.clone())
                } else {
                    Task::none()
                };
                (scroll_task, focus_task, None)
            }
            input_view::Event::ExitFocus => {
                self.focused_message = None;
                scroll_view.close_focus_menu();
                (Task::none(), Task::none(), None)
            }
            input_view::Event::FocusAction(action) => {
                let focused_message = std::mem::take(&mut self.focused_message);

                let our_nick: Option<Nick> = matches!(
                    action,
                    input_view::FocusAction::OpenReactionModal
                )
                .then(|| {
                    kind.server()
                        .and_then(|s| clients.nickname(s))
                        .map(NickRef::to_owned)
                })
                .flatten();

                let focused_component = self
                    .focused_message
                    .as_ref()
                    .and_then(FocusedMessage::focused_component);

                let (exit_task, _) = input_view.update(
                    input_view::Message::ExitFocus,
                    false,
                    upstream,
                    clients,
                    history,
                    config,
                );

                let hkind: history::Kind = kind.into();
                let message = focused_message.and_then(|focused_message| {
                    let view = history.get_messages(&hkind, None, config)?;
                    view.old_messages
                        .iter()
                        .chain(view.new_messages.iter())
                        .find(|message| focused_message.is_match(message))
                        .copied()
                });

                let (scroll_task, input_task, context_event) = match action {
                    input_view::FocusAction::Reply => {
                        let result = message.and_then(|message| {
                            Some((
                                message.id.clone()?,
                                message.server_time,
                                message
                                    .target
                                    .source()
                                    .user()
                                    .map(|u| u.nickname().to_owned())?,
                            ))
                        });
                        if let Some((msgid, server_time, to_nick)) = result {
                            let (reply_task, _) = input_view.update(
                                input_view::Message::SetDraftReply {
                                    msgid,
                                    server_time,
                                    to_nick,
                                },
                                false,
                                upstream,
                                clients,
                                history,
                                config,
                            );
                            (Task::none(), reply_task, None)
                        } else {
                            (Task::none(), Task::none(), None)
                        }
                    }
                    action => {
                        let focus_target = message
                            .zip(focused_component)
                            .and_then(|(message, focused_component)| {
                                focused_component.focus_target(message)
                            });

                        let mut scroll_task = Task::none();

                        let context_message = match action {
                            input_view::FocusAction::Redact => message
                                .and_then(|message| message.id.clone())
                                .map(context_menu::Message::Redact),
                            input_view::FocusAction::OpenReactionModal => message
                                .and_then(|message| {
                                    let id = message.id.clone()?;
                                    let selected =
                                        message_view::selected_reactions(
                                            message,
                                            our_nick.as_ref().map(NickRef::from),
                                        );
                                    Some(
                                        context_menu::Message::OpenReactionModal(
                                            id, selected,
                                        ),
                                    )
                                }),
                            input_view::FocusAction::OpenLink => {
                                match focus_target {
                                    Some(FocusTarget::Url(url)) => {
                                        Some(context_menu::Message::OpenUrl(
                                            url.to_string(),
                                        ))
                                    }
                                    Some(FocusTarget::Channel(
                                        channel,
                                    )) => {
                                        if let Some(server) = kind.server() {
                                            let link = message::Link::Channel(
                                                server.clone(),
                                                target::Channel::from_str(
                                                    &channel,
                                                    clients.get_server_chantypes_or_default(server),
                                                    clients.get_server_casemapping_or_default(server),
                                                ),
                                                config
                                                .actions
                                                .buffer
                                                .click_channel_name.buffer_action(),
                                            );
                                            scroll_task = Task::done(
                                                scroll_view::Message::Link(link),
                                            );
                                        }
                                        None
                                    }
                                    Some(FocusTarget::User(
                                        user,
                                    )) => {
                                        if let Some(server) = kind.server() {
                                            let link = message::Link::User(
                                                server.clone(),
                                                user.clone(),
                                            );
                                            scroll_task = Task::done(
                                                scroll_view::Message::Link(link),
                                            );
                                        }
                                        None
                                    }
                                    None => None,
                                }
                            }
                            input_view::FocusAction::CopyText => {
                                match focus_target {
                                    Some(FocusTarget::Url(url)) => {
                                        Some(context_menu::Message::CopyText(
                                            url.to_string(),
                                        ))
                                    }
                                    Some(FocusTarget::Channel(
                                        channel,
                                    )) => Some(
                                        context_menu::Message::CopyText(channel),
                                    ),
                                    Some(FocusTarget::User(user)) => {
                                        Some(context_menu::Message::CopyText(
                                            user.nickname().to_string(),
                                        ))
                                    }
                                    // Copy the message text when no link is
                                    // focused.
                                    None => message.map(|message| {
                                        context_menu::Message::CopyText(
                                            message.text().into_owned(),
                                        )
                                    }),
                                }
                            }
                            input_view::FocusAction::Reply => None,
                        };

                        (
                            scroll_task,
                            Task::none(),
                            context_message.and_then(context_menu::update),
                        )
                    }
                };

                (
                    scroll_task,
                    Task::batch([exit_task, input_task]),
                    context_event,
                )
            }
            _ => unreachable!(),
        }
    }

    pub fn handle_scroll_event(
        &mut self,
        event: &scroll_view::Event,
        scroll_view: &mut scroll_view::State,
        input_view: &mut input_view::State,
        upstream: &buffer::Upstream,
        kind: scroll_view::Kind<'_>,
        clients: &mut client::Map,
        history: &mut history::Manager,
        previews: &preview::Collection,
        config: &Config,
        channels_context: &dyn context_menu::ChannelsContext,
    ) -> Option<(
        Task<scroll_view::Message>,
        Task<input_view::Message>,
        Option<context_menu::Event>,
    )> {
        match event {
            scroll_view::Event::ExitFocus => {
                let (exit_task, _) = input_view.update(
                    input_view::Message::ExitFocus,
                    false,
                    upstream,
                    clients,
                    history,
                    config,
                );
                let focus_task = input_view.focus();
                Some((Task::none(), Task::batch([exit_task, focus_task]), None))
            }
            scroll_view::Event::FocusAction(action) => {
                Some(self.handle_input_event(
                    input_view::Event::FocusAction(*action),
                    scroll_view,
                    input_view,
                    upstream,
                    kind,
                    clients,
                    history,
                    previews,
                    config,
                    channels_context,
                ))
            }
            scroll_view::Event::FocusContextAction(message) => {
                self.focused_message = None;
                let (exit_task, _) = input_view.update(
                    input_view::Message::ExitFocus,
                    false,
                    upstream,
                    clients,
                    history,
                    config,
                );
                Some((
                    Task::none(),
                    exit_task,
                    context_menu::update(message.clone()),
                ))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FocusedMessage {
    hash: message::Hash,
    server_time: DateTime<Utc>,
    is_user_message: bool,
    link_count: usize,
    focused_component: Option<FocusedComponent>,
}

impl FocusedMessage {
    pub fn new(message: &data::Message) -> Self {
        Self {
            hash: message.hash,
            server_time: message.server_time,
            is_user_message: matches!(
                message.target.source(),
                message::Source::User(_)
            ),
            link_count: message_focus_target_count(message),
            focused_component: None,
        }
    }

    pub fn is_match(&self, message: &data::Message) -> bool {
        message.hash == self.hash
    }

    pub fn hash(&self) -> message::Hash {
        self.hash
    }

    pub fn server_time(&self) -> &DateTime<Utc> {
        &self.server_time
    }

    pub fn focused_component(&self) -> Option<&FocusedComponent> {
        self.focused_component.as_ref()
    }

    pub fn has_focused_component(&self) -> bool {
        self.focused_component.is_some()
    }

    pub fn focus_next_component(&mut self) {
        let next_component = match self.focused_component {
            None => {
                if self.is_user_message {
                    Some(FocusedComponent::User)
                } else if self.link_count > 0 {
                    Some(FocusedComponent::Link { index: 0 })
                } else {
                    None
                }
            }
            Some(FocusedComponent::User) => {
                if self.link_count > 0 {
                    Some(FocusedComponent::Link { index: 0 })
                } else {
                    Some(FocusedComponent::User)
                }
            }
            Some(FocusedComponent::Link { index }) => {
                Some(FocusedComponent::Link {
                    index: (index + 1).min(self.link_count - 1),
                })
            }
        };

        self.focused_component = next_component;
    }

    pub fn focus_previous_component(&mut self) {
        let previous_component = match self.focused_component {
            None => None,
            Some(FocusedComponent::User) => None,
            Some(FocusedComponent::Link { index }) => {
                if index > 0 {
                    Some(FocusedComponent::Link { index: index - 1 })
                } else {
                    self.is_user_message.then_some(FocusedComponent::User)
                }
            }
        };

        self.focused_component = previous_component;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FocusedComponent {
    Link { index: usize },
    User,
}

impl FocusedComponent {
    pub(crate) fn focus_target(
        &self,
        message: &data::Message,
    ) -> Option<FocusTarget> {
        match self {
            FocusedComponent::User => {
                message.user().map(|user| FocusTarget::User(user.clone()))
            }
            FocusedComponent::Link { index } => {
                message_focus_target_at(message, *index)
            }
        }
    }

    pub fn link_index(&self) -> Option<usize> {
        match self {
            Self::Link { index } => Some(*index),
            Self::User => None,
        }
    }
}

/// A keyboard-focusable link target within a message: a URL or a channel
/// mention.
#[derive(Debug, Clone)]
pub(crate) enum FocusTarget {
    Url(url::Url),
    Channel(String),
    User(User),
}

/// Iterator over a message's focusable link fragments, in display order.
fn message_focus_target_fragments(
    message: &data::Message,
) -> impl Iterator<Item = &message::Fragment> {
    let fragments: &[message::Fragment] = match &message.content {
        data::message::Content::Fragments(fragments) => fragments,
        _ => &[],
    };

    fragments.iter().filter(|f| f.is_focus_target())
}

/// Number of separately-navigable link targets in a message, in display order.
fn message_focus_target_count(message: &data::Message) -> usize {
    message_focus_target_fragments(message).count()
}

/// The `index`-th focusable link target of a message, in display order.
pub(crate) fn message_focus_target_at(
    message: &data::Message,
    index: usize,
) -> Option<FocusTarget> {
    message_focus_target_fragments(message)
        .nth(index)
        .and_then(|fragment| match fragment {
            message::Fragment::Url(url, _) => {
                Some(FocusTarget::Url(url.clone()))
            }
            message::Fragment::Channel(channel) => {
                Some(FocusTarget::Channel(channel.clone()))
            }
            message::Fragment::User(user, _) => {
                Some(FocusTarget::User(user.clone()))
            }
            _ => None,
        })
}
