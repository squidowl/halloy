use chrono::{DateTime, Utc};
use data::shortcut::{FocusCommand, KeyBind};
use data::user::User;
use data::{
    Config, Server, buffer, client, history, message, metadata, preview, target,
};
use iced::advanced::widget;
use iced::advanced::widget::operation::focusable;
use iced::widget::{column, container};
use iced::{Length, Task};

use super::{context_menu, input_view, scroll_view};
use crate::widget::{Element, double_pass, message_content, on_key};
use crate::{Theme, theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct Manager {
    focused_message: Option<FocusedMessage>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            focused_message: None,
        }
    }

    pub fn focused(&self) -> &Option<FocusedMessage> {
        &self.focused_message
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

    pub fn has_menu(&self) -> bool {
        self.focused_message
            .as_ref()
            .is_some_and(FocusedMessage::has_menu)
    }

    pub fn close_menu(&mut self) {
        if let Some(focused_message) = self.focused_message.as_mut() {
            focused_message.close_menu();
        }
    }

    pub fn clear(&mut self) -> bool {
        let was_focused = self.focused_message.is_some();

        self.focused_message = None;

        was_focused
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
                );

                if let Some(scroll_view::Event::ExitFocus(context_menu_event)) =
                    scroll_event
                {
                    let (input_task, _) = input_view.update(
                        input_view::Message::ExitFocus,
                        false,
                        upstream,
                        clients,
                        history,
                        config,
                    );

                    return (scroll_task, input_task, context_menu_event);
                }

                let entered_mode =
                    !was_in_mode && self.focused_message.is_some();

                let focus_task = if entered_mode {
                    widget::operate(focusable::unfocus())
                } else {
                    Task::none()
                };

                (scroll_task, focus_task, None)
            }
            input_view::Event::ExitFocus => {
                self.focused_message = None;

                (Task::none(), Task::none(), None)
            }
            input_view::Event::FocusAction(action) => {
                let focused_message = self.focused_message.take();

                let focused_component = focused_message
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

                let message =
                    focused_message.as_ref().and_then(|focused_message| {
                        history.find_message_by_hash(
                            focused_message.hash(),
                            &kind.into(),
                            focused_message.server_time(),
                        )
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
                                    Some(
                                        context_menu::Message::OpenReactionModal(
                                            id, message.server_time,
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
    ) -> Option<(
        Task<scroll_view::Message>,
        Task<input_view::Message>,
        Option<context_menu::Event>,
    )> {
        match event {
            scroll_view::Event::ExitFocus(context_menu_event) => {
                let (exit_task, _) = input_view.update(
                    input_view::Message::ExitFocus,
                    false,
                    upstream,
                    clients,
                    history,
                    config,
                );

                Some((Task::none(), exit_task, context_menu_event.clone()))
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

#[derive(Debug, Clone)]
pub struct FocusedMessage {
    hash: message::Hash,
    server_time: DateTime<Utc>,
    is_user_message: bool,
    focusable_fragment_indices: Vec<usize>,
    focused_component: Option<FocusedComponent>,
    menu: Option<FocusMenu>,
}

impl FocusedMessage {
    pub fn new(message: &data::Message, config: &Config) -> Self {
        Self {
            hash: message.hash,
            server_time: message.server_time,
            is_user_message: matches!(
                message.target.source(),
                message::Source::User(_)
            ),
            focusable_fragment_indices: message_focus_target_indices(
                message, config,
            ),
            focused_component: None,
            menu: None,
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
        // Moving the focus dismisses any open action menu
        self.menu = None;

        let first_link = self
            .focusable_fragment_indices
            .first()
            .copied()
            .map(|index| FocusedComponent::Link { index });

        let next_component = match self.focused_component {
            None => {
                if self.is_user_message {
                    Some(FocusedComponent::User)
                } else {
                    first_link
                }
            }
            Some(FocusedComponent::User) => {
                first_link.or(Some(FocusedComponent::User))
            }
            Some(FocusedComponent::Link { index }) => self
                .focusable_fragment_indices
                .iter()
                .copied()
                .find(|candidate| *candidate > index)
                .map(|index| FocusedComponent::Link { index })
                .or(self.focused_component),
        };

        self.focused_component = next_component;
    }

    pub fn focus_previous_component(&mut self) {
        // Moving the focus dismisses any open action menu
        self.menu = None;

        let previous_component = match self.focused_component {
            None => None,
            Some(FocusedComponent::User) => None,
            Some(FocusedComponent::Link { index }) => self
                .focusable_fragment_indices
                .iter()
                .copied()
                .rev()
                .find(|candidate| *candidate < index)
                .map(|index| FocusedComponent::Link { index })
                .or_else(|| {
                    self.is_user_message.then_some(FocusedComponent::User)
                }),
        };

        self.focused_component = previous_component;
    }

    pub fn open_menu(
        &mut self,
        message: &data::Message,
        server: &Server,
        clients: &client::Map,
        previews: &preview::Collection,
        config: &Config,
    ) -> Task<scroll_view::Message> {
        let focus_target =
            self.focused_component.and_then(|focused_component| {
                focused_component.focus_target(message)
            });

        let (menu, on_open_task) = match focus_target {
            Some(FocusTarget::Url(url)) => (
                FocusMenu {
                    link: Some(message::Link::Url(url.as_str().to_string())),
                    ..FocusMenu::default()
                },
                Task::none(),
            ),
            Some(FocusTarget::Channel(channel)) => {
                let channel = target::Channel::from_str(
                    &channel,
                    clients.get_server_chantypes_or_default(server),
                    clients.get_server_casemapping_or_default(server),
                );

                (
                    FocusMenu {
                        link: Some(message::Link::Channel(
                            server.clone(),
                            channel,
                            config
                                .actions
                                .buffer
                                .click_channel_name
                                .buffer_action(),
                        )),
                        ..FocusMenu::default()
                    },
                    Task::none(),
                )
            }
            Some(FocusTarget::User(user)) => {
                let on_open_task = config
                    .metadata
                    .avatar_size()
                    .filter(|_| config.context_menu.show_user_metadata)
                    .and_then(|size| {
                        metadata::avatar_url(
                            &user,
                            clients.get_registry(server),
                            size,
                        )
                    })
                    .filter(|url| !previews.contains_key(url))
                    .map_or(Task::none(), |url| {
                        Task::done(scroll_view::Message::ContextMenu(
                            context_menu::Message::LoadUserAvatar(
                                server.clone(),
                                url.clone(),
                            ),
                        ))
                    });

                (
                    FocusMenu {
                        link: Some(message::Link::User(server.clone(), user)),
                        ..FocusMenu::default()
                    },
                    on_open_task,
                )
            }
            // Parent message is focused
            None => (FocusMenu::default(), Task::none()),
        };

        self.menu = Some(menu);

        on_open_task
    }

    pub fn menu(&self) -> Option<&FocusMenu> {
        self.menu.as_ref()
    }

    pub fn has_menu(&self) -> bool {
        self.menu.is_some()
    }

    pub fn menu_select(&mut self, selection: usize) {
        if let Some(menu) = self.menu.as_mut() {
            menu.selection = selection;
        }
    }

    pub fn close_menu(&mut self) {
        self.menu = None;
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

    pub fn fragment_index(&self) -> Option<usize> {
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

/// Indices of the message fragments that are both focusable and rendered.
fn message_focus_target_indices(
    message: &data::Message,
    config: &Config,
) -> Vec<usize> {
    let fragments: &[message::Fragment] = match &message.content {
        data::message::Content::Fragments(fragments) => fragments,
        _ => &[],
    };

    if matches!(message.target.source(), message::Source::User(_))
        && message.redaction_expanded(&config.buffer.redaction) == Some(false)
    {
        return vec![];
    }

    let prefix_skip_until =
        if matches!(message.target.source(), message::Source::User(_))
            && config.buffer.reply.hide_redundant_nicks
        {
            message
                .reply_preview
                .as_ref()
                .and_then(|reply_preview| reply_preview.user.as_ref())
                .map(|user| {
                    message_content::leading_nick_skip(fragments, user.as_str())
                })
                .unwrap_or_default()
        } else {
            0
        };

    fragments
        .iter()
        .enumerate()
        .skip(prefix_skip_until)
        .filter_map(|(index, fragment)| {
            fragment.is_focus_target().then_some(index)
        })
        .collect()
}

/// The focusable target at `index` in the message's fragment collection.
pub(crate) fn message_focus_target_at(
    message: &data::Message,
    index: usize,
) -> Option<FocusTarget> {
    match &message.content {
        data::message::Content::Fragments(fragments) => fragments.get(index),
        _ => None,
    }
    .and_then(|fragment| match fragment {
        message::Fragment::Url(url, _) => Some(FocusTarget::Url(url.clone())),
        message::Fragment::Channel(channel) => {
            Some(FocusTarget::Channel(channel.clone()))
        }
        message::Fragment::User(user, _) => {
            Some(FocusTarget::User(user.clone()))
        }
        _ => None,
    })
}

/// A keyboard-navigable menu of focus actions, anchored to a focused message.
#[derive(Debug, Clone, Default)]
pub struct FocusMenu {
    selection: usize,
    link: Option<message::Link>,
}

impl FocusMenu {
    pub fn link(&self) -> Option<&message::Link> {
        self.link.as_ref()
    }
}

/// Renders the open focus action menu. Anchored by the caller to the menu's
/// target (the nick or the message content) within the message layout.
pub fn focus_menu_overlay<'a>(
    menu: &'a FocusMenu,
    entries: Vec<context_menu::Entry>,
    context: context_menu::Context,
    theme: &'a Theme,
    config: &'a Config,
) -> Element<'a, scroll_view::Message> {
    let entry_messages = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            entry.as_label_and_message(&context, config).and_then(
                |(_, message)| message.map(|message| (index, message)),
            )
        })
        .collect::<Vec<_>>();

    let capped_selection =
        menu.selection.min(entry_messages.len().saturating_sub(1));

    let selection = entry_messages.get(capped_selection);

    let build = |width: Length| -> Element<'a, scroll_view::Message> {
        let entries = entries.iter().enumerate().fold(
            column![],
            |col, (index, entry)| {
                // Reuse the right-click row rendering (incl. selection
                // highlight); route any click through the same activation path
                // as keyboard Enter so focus mode is exited consistently.
                let element = (*entry)
                    .view(
                        Some(context.clone()),
                        width,
                        config,
                        theme,
                        selection.is_some_and(|(selection_index, _)| {
                            index == *selection_index
                        }),
                    )
                    .map(|message| {
                        scroll_view::Message::FocusMenuActivate(message)
                    });

                col.push(element)
            },
        );

        container(entries)
            .padding(4)
            .style(theme::container::tooltip)
            .into()
    };

    let panel = double_pass(build(Length::Shrink), build(Length::Fill));

    let up_index = if entry_messages.is_empty() {
        0
    } else {
        (capped_selection + entry_messages.len() - 1) % entry_messages.len()
    };

    let down_index = if entry_messages.is_empty() {
        0
    } else {
        (capped_selection + 1) % entry_messages.len()
    };

    let selection_message = selection.map(|(_, message)| {
        scroll_view::Message::FocusMenuActivate(message.clone())
    });

    on_key(panel, move |key, modifiers| {
        let key_bind = KeyBind::from((key.clone(), modifiers));

        config.keyboard.focus_command(&key_bind, true).and_then(
            |focus_command| match focus_command {
                FocusCommand::Up => {
                    Some(scroll_view::Message::FocusMenuSelect(up_index))
                }
                FocusCommand::Down => {
                    Some(scroll_view::Message::FocusMenuSelect(down_index))
                }
                FocusCommand::Activate | FocusCommand::ActivateAlt => {
                    selection_message.clone()
                }
                FocusCommand::Left
                | FocusCommand::Right
                | FocusCommand::Reply
                | FocusCommand::React
                | FocusCommand::Redact => None,
            },
        )
    })
}

pub(crate) fn focus_outline<'a, Message: 'a>(
    inner: Element<'a, Message>,
) -> Element<'a, Message> {
    use iced::advanced::{Layout, Renderer as _, mouse, renderer, widget};

    crate::widget::decorate(inner)
        .draw(
            move |_state: &(),
                  inner: &Element<'a, Message>,
                  tree: &widget::Tree,
                  renderer: &mut crate::widget::Renderer,
                  theme: &Theme,
                  style: &renderer::Style,
                  layout: Layout<'_>,
                  cursor: mouse::Cursor,
                  viewport: &iced::Rectangle| {
                let layout_bounds = layout.bounds();
                let focus_outline_bounds = iced::Rectangle {
                    x: layout_bounds.x - 2.0,
                    y: layout_bounds.y - 2.0,
                    width: layout_bounds.width + 4.0,
                    height: layout_bounds.height + 4.0,
                };

                renderer.fill_quad(
                    renderer::Quad {
                        bounds: focus_outline_bounds,
                        ..renderer::Quad::default()
                    },
                    theme.styles().buffer.background_text_input,
                );

                inner.as_widget().draw(
                    tree, renderer, theme, style, layout, cursor, viewport,
                );

                renderer.fill_quad(
                    renderer::Quad {
                        bounds: focus_outline_bounds,
                        border: theme::focus_border(theme),
                        ..renderer::Quad::default()
                    },
                    iced::Color::TRANSPARENT,
                );
            },
        )
        .into()
}
