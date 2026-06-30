use data::user::{Nick, NickRef};
use data::{Config, buffer, client, history, message, target};
use iced::{Task, widget};

use super::{context_menu, input_view, message_view, scroll_view};
use crate::window::Window;

#[derive(Debug, Clone)]
pub struct Manager {
    focus_capture_id: widget::Id,
    focused_message: Option<message::Hash>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            focus_capture_id: widget::Id::unique(),
            focused_message: None,
        }
    }

    pub fn focused(&self) -> Option<message::Hash> {
        self.focused_message
    }

    pub fn is_focused(&self) -> bool {
        self.focused_message.is_some()
    }

    pub fn focused_mut(&mut self) -> &mut Option<message::Hash> {
        &mut self.focused_message
    }

    pub fn clear(&mut self) {
        self.focused_message = None;
    }

    // Returns a zero-size hidden text_input. While a message is focused this widget
    // holds keyboard focus so that Alt+Arrow events reach the global subscription
    // rather than being consumed by the text editor.
    pub fn focus_capture<'a, M: Clone + 'a>(
        &self,
    ) -> crate::widget::Element<'a, M> {
        widget::container(
            widget::text_input("", "")
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
        main_window: &Window,
        config: &Config,
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
                    config,
                    None,
                );
                if let Some(scroll_view::Event::ExitFocus) = scroll_event {
                    let (exit_task, _) = input_view.update(
                        input_view::Message::ExitFocus,
                        false,
                        upstream,
                        clients,
                        history,
                        main_window,
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
                let hash = self.focused_message;
                self.focused_message = None;

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

                let focused_link = scroll_view.focused_link();

                let (exit_task, _) = input_view.update(
                    input_view::Message::ExitFocus,
                    false,
                    upstream,
                    clients,
                    history,
                    main_window,
                    config,
                );

                let hkind: history::Kind = kind.into();
                let message = hash.and_then(|h| {
                    let view = history.get_messages(&hkind, None, config)?;
                    view.old_messages
                        .iter()
                        .chain(view.new_messages.iter())
                        .find(|m| m.hash == h)
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
                                main_window,
                                config,
                            );
                            (Task::none(), reply_task, None)
                        } else {
                            (Task::none(), Task::none(), None)
                        }
                    }
                    action => {
                        let focus_target = message.and_then(|message| {
                            focused_link
                                .and_then(|index| {
                                    scroll_view::message_focus_target_at(
                                        message, index,
                                    )
                                })
                                .or_else(|| {
                                    scroll_view::message_single_url(message)
                                        .map(scroll_view::FocusTarget::Url)
                                })
                        });

                        let mut scroll_task = Task::none();

                        let context_message = match action {
                            input_view::FocusAction::CopyText => {
                                message.map(|message| {
                                    context_menu::Message::CopyText(
                                        message.text().into_owned(),
                                    )
                                })
                            }
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
                            input_view::FocusAction::OpenUrl => {
                                match focus_target {
                                    Some(scroll_view::FocusTarget::Url(url)) => {
                                        Some(context_menu::Message::OpenUrl(
                                            url.to_string(),
                                        ))
                                    }
                                    Some(scroll_view::FocusTarget::Channel(
                                        channel,
                                    )) => {
                                        if let Some(server) = kind.server() {
                                            let buffer_action = match config
                                                .actions
                                                .buffer
                                                .click_channel_name
                                            {
                                                data::config::actions::ChannelClickAction::OpenChannel(
                                                    buffer_action,
                                                ) => buffer_action,
                                                data::config::actions::ChannelClickAction::Noop => {
                                                    data::dashboard::BufferAction::default()
                                                }
                                            };
                                            let link = message::Link::Channel(
                                                server.clone(),
                                                target::Channel::from_str(
                                                    &channel,
                                                    clients.get_server_chantypes_or_default(server),
                                                    clients.get_server_casemapping_or_default(server),
                                                ),
                                                buffer_action,
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
                            input_view::FocusAction::CopyUrl => {
                                match focus_target {
                                    Some(scroll_view::FocusTarget::Url(url)) => {
                                        Some(context_menu::Message::CopyUrl(
                                            url.to_string(),
                                        ))
                                    }
                                    Some(scroll_view::FocusTarget::Channel(
                                        channel,
                                    )) => Some(
                                        context_menu::Message::CopyText(channel),
                                    ),
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
        main_window: &Window,
        config: &Config,
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
                    main_window,
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
                    main_window,
                    config,
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
                    main_window,
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
