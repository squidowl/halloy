use data::user::{Nick, NickRef};
use data::{Config, buffer, client, history, message};
use iced::{Task, widget};

use super::{context_menu, input_view, scroll_view};
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
            input_view::Event::ScrollToBottom => {
                let mut scroll_task = scroll_view.scroll_to_end(config);

                if config.buffer.mark_as_read.on_scroll_to_bottom {
                    scroll_task = scroll_task
                        .chain(Task::done(scroll_view::Message::MarkAsRead));
                }

                (scroll_task, Task::none(), None)
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

                let focused_url = scroll_view.focused_url();

                let msg_data = hash.and_then(|h| {
                    let hkind: history::Kind = kind.into();
                    let view = history.get_messages(&hkind, None, config)?;
                    let msg = view
                        .old_messages
                        .iter()
                        .chain(view.new_messages.iter())
                        .find(|m| m.hash == h)?;
                    Some((
                        msg.id.clone(),
                        msg.server_time,
                        msg.target
                            .source()
                            .user()
                            .map(|u| u.nickname().to_owned()),
                        msg.text().into_owned(),
                        msg.reactions
                            .iter()
                            .map(|r| {
                                (r.sender.clone(), r.text.clone(), r.unreact)
                            })
                            .collect::<Vec<_>>(),
                        // Resolve the focused URL the same way the action menu
                        // does: the navigated link, falling back to a message
                        // that is itself a single URL.
                        focused_url
                            .and_then(|index| {
                                scroll_view::message_url_at(msg, index)
                            })
                            .or_else(|| scroll_view::message_single_url(msg))
                            .map(|url| url.to_string()),
                    ))
                });

                let (exit_task, _) = input_view.update(
                    input_view::Message::ExitFocus,
                    false,
                    upstream,
                    clients,
                    history,
                    main_window,
                    config,
                );

                let (context_event, extra_task) = match action {
                    input_view::FocusAction::Reply => {
                        let result = msg_data.as_ref().and_then(
                            |(id, server_time, to_nick, ..)| {
                                Some((
                                    id.clone()?,
                                    *server_time,
                                    to_nick.clone()?,
                                ))
                            },
                        );
                        if let Some((msgid, server_time, to_nick)) = result {
                            let (reply_task, _) = input_view.update(
                                input_view::Message::SetDraftReply {
                                    msgid: msgid.clone(),
                                    server_time,
                                    to_nick: to_nick.clone(),
                                },
                                false,
                                upstream,
                                clients,
                                history,
                                main_window,
                                config,
                            );
                            (None, reply_task)
                        } else {
                            (None, Task::none())
                        }
                    }
                    input_view::FocusAction::Redact => (
                        msg_data
                            .as_ref()
                            .and_then(|(id, ..)| id.clone())
                            .map(context_menu::Event::RedactMessage),
                        Task::none(),
                    ),
                    input_view::FocusAction::CopyText => (
                        msg_data.as_ref().map(|(_, _, _, text, _, _)| {
                            context_menu::Event::CopyText(text.clone())
                        }),
                        Task::none(),
                    ),
                    input_view::FocusAction::OpenUrl => (
                        msg_data.as_ref().and_then(
                            |(_, _, _, _, _, focus_url)| {
                                focus_url
                                    .clone()
                                    .map(context_menu::Event::OpenUrl)
                            },
                        ),
                        Task::none(),
                    ),
                    input_view::FocusAction::CopyUrl => (
                        // Bound to the copy shortcut: copy the focused link, or
                        // the message text when no link is focused.
                        msg_data.as_ref().map(
                            |(_, _, _, text, _, focus_url)| {
                                focus_url.clone().map_or_else(
                                    || {
                                        context_menu::Event::CopyText(
                                            text.clone(),
                                        )
                                    },
                                    context_menu::Event::CopyUrl,
                                )
                            },
                        ),
                        Task::none(),
                    ),
                    input_view::FocusAction::OpenReactionModal => (
                        msg_data.as_ref().and_then(
                            |(id, _, _, _, reactions, _)| {
                                let id = id.clone()?;
                                let selected = our_nick
                                    .as_ref()
                                    .map(|nick| {
                                        scroll_view::active_reactions_for_nick(
                                            nick, reactions,
                                        )
                                    })
                                    .unwrap_or_default();
                                Some(context_menu::Event::OpenReactionModal(
                                    id, selected,
                                ))
                            },
                        ),
                        Task::none(),
                    ),
                };

                (
                    Task::none(),
                    Task::batch([exit_task, extra_task]),
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
