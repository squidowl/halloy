use std::path::PathBuf;

use chrono::{DateTime, Utc};
use data::dashboard::BufferAction;
use data::preview::{self, Previews};
use data::server::Server;
use data::target::{self, Target};
use data::user::{ChannelUsers, Nick};
use data::{Config, Preview, User, buffer, client, history, message};
use iced::widget::{column, container, row};
use iced::{Length, Size, Task, padding};

use super::message_view::{ChannelQueryLayout, TargetInfo};
use super::{context_menu, input_view, scroll_view};
use crate::Theme;
use crate::widget::Element;

mod topic;

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
    ContextMenu(context_menu::Message),
    Topic(topic::Message),
}

pub enum Event {
    ContextMenu(context_menu::Event),
    OpenBuffers(Vec<(Target, BufferAction)>),
    LeaveBuffers(Vec<Target>, Option<String>),
    History(Task<history::manager::Message>),
    RequestOlderChatHistory,
    PreviewChanged,
    HidePreview(history::Kind, message::Hash, url::Url),
    MarkAsRead(history::Kind),
    OpenUrl(String),
    ImagePreview(PathBuf, url::Url),
    ExpandCondensedMessage(DateTime<Utc>, message::Hash),
    ContractCondensedMessage(DateTime<Utc>, message::Hash),
}

pub fn view<'a>(
    state: &'a Channel,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    previews: &'a preview::Collection,
    settings: Option<&'a buffer::Settings>,
    config: &'a Config,
    theme: &'a Theme,
    is_focused: bool,
) -> Element<'a, Message> {
    let server = &state.server;
    let connected = matches!(clients.status(server), client::Status::Connected);
    let chantypes = clients.get_chantypes(server);
    let casemapping = clients.get_casemapping(server);
    let prefix = clients.get_prefix(server);
    let supports_echoes = clients.get_server_supports_echoes(server);
    let channel = &state.target;
    let buffer = &state.buffer;
    let input = history.input(buffer);
    let our_nick: Option<data::user::NickRef<'_>> =
        clients.nickname(&state.server);

    let our_user = our_nick
        .map(|our_nick| User::from(Nick::from(our_nick)))
        .and_then(|user| {
            clients.resolve_user_attributes(&state.server, channel, &user)
        });

    let users = clients.get_channel_users(&state.server, channel);

    let chathistory_state =
        clients.get_chathistory_state(server, &channel.to_target());

    let previews = Some(Previews::new(
        previews,
        &channel.to_target(),
        server,
        &config.preview,
        casemapping,
    ));

    let message_formatter = ChannelQueryLayout {
        config,
        chantypes,
        casemapping,
        prefix,
        supports_echoes,
        connected,
        server,
        theme,
        target: TargetInfo::Channel {
            users,
            channel,
            our_user,
        },
    };

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Channel(&state.server, channel),
            history,
            previews,
            Some(|preview: &Preview, source: &message::Source| {
                preview.visible_for_source(
                    source,
                    Some(channel),
                    Some(server),
                    casemapping,
                    &config.preview,
                )
            }),
            chathistory_state,
            config,
            theme,
            message_formatter,
        )
        .map(Message::ScrollView),
    )
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    let nick_list = nick_list::view(
        server, prefix, channel, users, our_user, config, theme,
    )
    .map(Message::ContextMenu);

    // If topic toggles from None to Some then it messes with messages' scroll state,
    // so produce a zero-height placeholder when topic is None.
    let topic = topic(state, clients, users, our_user, settings, config, theme)
        .unwrap_or_else(|| column![].into());

    let show_text_input = match config.buffer.text_input.visibility {
        data::buffer::TextInputVisibility::Focused => is_focused,
        data::buffer::TextInputVisibility::Always => true,
    };

    let mut channels = clients.get_channels(&state.server);
    let is_connected_to_channel = channels.any(|c| c == &state.target);

    let text_input = show_text_input.then(move || {
        input_view::view(
            &state.input_view,
            input,
            is_focused,
            our_user,
            !is_connected_to_channel,
            config,
            theme,
        )
        .map(Message::InputView)
    });

    let content = column![topic, messages];

    let nicklist_enabled = settings
        .map_or(config.buffer.channel.nicklist.enabled, |settings| {
            settings.channel.nicklist.enabled
        });

    let content =
        match (nicklist_enabled, config.buffer.channel.nicklist.position) {
            (true, data::channel::Position::Left) => row![nick_list, content],
            (true, data::channel::Position::Right) => row![content, nick_list],
            (false, _) => { row![content] }.height(Length::Fill),
        }
        .spacing(4)
        .padding(padding::left(8).right(8));

    let body = column![container(content).height(Length::Fill), text_input]
        .height(Length::Fill);

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub buffer: buffer::Upstream,
    pub server: Server,
    pub target: target::Channel,
    pub scroll_view: scroll_view::State,
    pub input_view: input_view::State,
}

impl Channel {
    pub fn new(
        server: Server,
        target: target::Channel,
        pane_size: Size,
        config: &Config,
    ) -> Self {
        Self {
            buffer: buffer::Upstream::Channel(server.clone(), target.clone()),
            server,
            target,
            scroll_view: scroll_view::State::new(pane_size, config),
            input_view: input_view::State::new(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(
                    message,
                    config.buffer.chathistory.infinite_scroll,
                    scroll_view::Kind::Channel(&self.server, &self.target),
                    history,
                    clients,
                    config,
                );

                let event = event.and_then(|event| match event {
                    scroll_view::Event::ContextMenu(event) => {
                        Some(Event::ContextMenu(event))
                    }
                    scroll_view::Event::OpenBuffer(target, buffer_action) => {
                        Some(Event::OpenBuffers(vec![(target, buffer_action)]))
                    }
                    scroll_view::Event::GoToMessage(..) => None,
                    scroll_view::Event::RequestOlderChatHistory => {
                        Some(Event::RequestOlderChatHistory)
                    }
                    scroll_view::Event::PreviewChanged => {
                        Some(Event::PreviewChanged)
                    }
                    scroll_view::Event::HidePreview(kind, hash, url) => {
                        Some(Event::HidePreview(kind, hash, url))
                    }
                    scroll_view::Event::MarkAsRead => {
                        history::Kind::from_buffer(data::Buffer::Upstream(
                            self.buffer.clone(),
                        ))
                        .map(Event::MarkAsRead)
                    }
                    scroll_view::Event::OpenUrl(url) => {
                        Some(Event::OpenUrl(url))
                    }
                    scroll_view::Event::ImagePreview(path, url) => {
                        Some(Event::ImagePreview(path, url))
                    }
                    scroll_view::Event::ExpandCondensedMessage(
                        server_time,
                        hash,
                    ) => Some(Event::ExpandCondensedMessage(server_time, hash)),
                    scroll_view::Event::ContractCondensedMessage(
                        server_time,
                        hash,
                    ) => {
                        Some(Event::ContractCondensedMessage(server_time, hash))
                    }
                });

                (command.map(Message::ScrollView), event)
            }
            Message::InputView(message) => {
                let (command, event) = self.input_view.update(
                    message,
                    &self.buffer,
                    clients,
                    history,
                    config,
                );
                let command = command.map(Message::InputView);

                match event {
                    Some(input_view::Event::InputSent { history_task }) => {
                        let command = Task::batch(vec![
                            command,
                            self.scroll_view
                                .scroll_to_end(config)
                                .map(Message::ScrollView),
                        ]);

                        (command, Some(Event::History(history_task)))
                    }
                    Some(input_view::Event::OpenBuffers { targets }) => {
                        (command, Some(Event::OpenBuffers(targets)))
                    }
                    Some(input_view::Event::LeaveBuffers {
                        targets,
                        reason,
                    }) => (command, Some(Event::LeaveBuffers(targets, reason))),
                    Some(input_view::Event::Cleared { history_task }) => {
                        (command, Some(Event::History(history_task)))
                    }
                    None => (command, None),
                }
            }
            Message::ContextMenu(message) => (
                Task::none(),
                Some(Event::ContextMenu(context_menu::update(message))),
            ),
            Message::Topic(message) => (
                Task::none(),
                topic::update(message).map(|event| match event {
                    topic::Event::ContextMenu(event) => {
                        Event::ContextMenu(event)
                    }
                    topic::Event::OpenChannel(channel) => {
                        Event::OpenBuffers(vec![(
                            Target::Channel(channel),
                            config.actions.buffer.click_channel_name,
                        )])
                    }
                    topic::Event::OpenUrl(url) => Event::OpenUrl(url),
                }),
            ),
        }
    }

    pub fn focus(&self) -> Task<Message> {
        self.input_view.focus().map(Message::InputView)
    }

    pub fn reset(&mut self) {
        self.input_view.reset();
    }
}

fn topic<'a>(
    state: &'a Channel,
    clients: &'a data::client::Map,
    users: Option<&'a ChannelUsers>,
    our_user: Option<&'a User>,
    settings: Option<&'a buffer::Settings>,
    config: &'a Config,
    theme: &'a Theme,
) -> Option<Element<'a, Message>> {
    let topic_enabled = settings
        .map_or(config.buffer.channel.topic_banner.enabled, |settings| {
            settings.channel.topic_banner.enabled
        });

    if !topic_enabled {
        return None;
    }

    let chantypes = clients.get_chantypes(&state.server);
    let casemapping = clients.get_casemapping(&state.server);
    let prefix = clients.get_prefix(&state.server);

    let topic = clients.get_channel_topic(&state.server, &state.target)?;

    Some(
        topic::view(
            &state.server,
            chantypes,
            casemapping,
            prefix,
            &state.target,
            topic.content.as_ref()?,
            topic.who.as_ref().map(Nick::as_nickref),
            topic.time.as_ref(),
            config.buffer.channel.topic_banner.max_lines,
            users,
            our_user,
            config,
            theme,
        )
        .map(Message::Topic),
    )
}

mod nick_list {
    use context_menu::Message;
    use data::user::ChannelUsers;
    use data::{Config, Server, User, config, isupport, target};
    use iced::Length;
    use iced::advanced::text;
    use iced::widget::{Scrollable, column, scrollable};

    use crate::buffer::context_menu;
    use crate::widget::{Element, selectable_text};
    use crate::{Theme, font, theme};

    pub fn view<'a>(
        server: &'a Server,
        prefix: &'a [isupport::PrefixMap],
        channel: &'a target::Channel,
        users: Option<&'a ChannelUsers>,
        our_user: Option<&'a User>,
        config: &'a Config,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let nicklist_config = &config.buffer.channel.nicklist;

        let width = match nicklist_config.width {
            Some(width) => width,
            None => {
                let max_nick_length = users
                    .into_iter()
                    .flatten()
                    .map(|user| {
                        user.display(nicklist_config.show_access_levels, None)
                            .chars()
                            .count()
                    })
                    .max()
                    .unwrap_or_default();

                font::width_from_chars(max_nick_length, &config.font)
            }
        };

        let content = column(users.into_iter().flatten().map(|user| {
            let content = selectable_text(
                user.display(nicklist_config.show_access_levels, None),
            )
            .font_maybe(
                theme::font_style::nickname(theme, false).map(font::get),
            )
            .style(|theme| {
                theme::selectable_text::nicklist_nickname(theme, config, user)
            })
            .align_x(match nicklist_config.alignment {
                config::buffer::channel::Alignment::Left => {
                    text::Alignment::Left
                }
                config::buffer::channel::Alignment::Right => {
                    text::Alignment::Right
                }
            })
            .width(Length::Fixed(width));

            context_menu::user(
                content,
                server,
                prefix,
                Some(channel),
                user,
                Some(user),
                our_user,
                config,
                theme,
                &config.buffer.channel.nicklist.click,
            )
        }));

        Scrollable::new(content)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(1).scroller_width(1),
            ))
            .width(Length::Shrink)
            .style(theme::scrollable::hidden)
            .into()
    }
}
