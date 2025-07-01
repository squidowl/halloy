use std::path::PathBuf;

use data::dashboard::BufferAction;
use data::preview::{self, Previews};
use data::server::Server;
use data::target::{self, Target};
use data::user::Nick;
use data::{Config, User, buffer, history, message};
use iced::widget::{column, container, row};
use iced::{Length, Task, padding};

use super::message_view::{ChannelQueryLayout, TargetInfo};
use super::{input_view, scroll_view, user_context};
use crate::Theme;
use crate::widget::Element;

mod topic;

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
    UserContext(user_context::Message),
    Topic(topic::Message),
}

pub enum Event {
    UserContext(user_context::Event),
    OpenBuffers(Vec<(Target, BufferAction)>),
    History(Task<history::manager::Message>),
    RequestOlderChatHistory,
    PreviewChanged,
    HidePreview(history::Kind, message::Hash, url::Url),
    MarkAsRead(history::Kind),
    OpenUrl(String),
    ImagePreview(PathBuf, url::Url),
    Reacted {
        msgid: message::Id,
        text: String,
        unreacted: bool,
    },
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
    let casemapping = clients.get_casemapping(server);
    let channel = &state.target;
    let buffer = &state.buffer;
    let input = history.input(buffer);
    let our_nick = clients.nickname(&state.server);

    let our_user = our_nick
        .map(|our_nick| User::from(Nick::from(our_nick.as_ref())))
        .and_then(|user| {
            clients.resolve_user_attributes(&state.server, channel, &user)
        });

    let users = clients.get_channel_users(&state.server, channel);

    let chathistory_state =
        clients.get_chathistory_state(server, &channel.to_target());

    let previews = Some(Previews::new(
        previews,
        &channel.to_target(),
        &config.preview,
        casemapping,
    ));

    let message_formatter = ChannelQueryLayout {
        config,
        casemapping,
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
            chathistory_state,
            config,
            message_formatter,
        )
        .map(Message::ScrollView),
    )
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    let nick_list =
        nick_list::view(server, casemapping, channel, users, our_user, config)
            .map(Message::UserContext);

    // If topic toggles from None to Some then it messes with messages' scroll state,
    // so produce a zero-height placeholder when topic is None.
    let topic = topic(state, clients, users, our_user, settings, config, theme)
        .unwrap_or_else(|| column![].into());

    let show_text_input = match config.buffer.text_input.visibility {
        data::buffer::TextInputVisibility::Focused => is_focused,
        data::buffer::TextInputVisibility::Always => true,
    };

    let channels = clients.get_channels(&state.server);
    let is_connected_to_channel = channels.iter().any(|c| c == &state.target);

    let text_input = show_text_input.then(move || {
        input_view::view(
            &state.input_view,
            input,
            is_focused,
            !is_connected_to_channel,
            config,
        )
        .map(Message::InputView)
    });

    let content = column![topic, messages].spacing(4);

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
        .spacing(4);

    let body = column![]
        .push(container(content).height(Length::Fill))
        .push_maybe(text_input)
        .spacing(4)
        .height(Length::Fill);

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(padding::all(8).top(4))
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
    pub fn new(server: Server, target: target::Channel) -> Self {
        Self {
            buffer: buffer::Upstream::Channel(server.clone(), target.clone()),
            server,
            target,
            scroll_view: scroll_view::State::new(),
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
                    scroll_view::Event::UserContext(event) => {
                        Some(Event::UserContext(event))
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
                    scroll_view::Event::Reacted {
                        msgid,
                        text,
                        unreacted,
                    } => Some(Event::Reacted {
                        msgid,
                        text,
                        unreacted,
                    }),
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
                                .scroll_to_end()
                                .map(Message::ScrollView),
                        ]);

                        (command, Some(Event::History(history_task)))
                    }
                    Some(input_view::Event::OpenBuffers { targets }) => {
                        (command, Some(Event::OpenBuffers(targets)))
                    }
                    None => (command, None),
                }
            }
            Message::UserContext(message) => (
                Task::none(),
                Some(Event::UserContext(user_context::update(message))),
            ),
            Message::Topic(message) => (
                Task::none(),
                topic::update(message).map(|event| match event {
                    topic::Event::UserContext(event) => {
                        Event::UserContext(event)
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
    users: &'a [User],
    our_user: Option<&'a User>,
    settings: Option<&'a buffer::Settings>,
    config: &'a Config,
    theme: &'a Theme,
) -> Option<Element<'a, Message>> {
    let topic_enabled = settings
        .map_or(config.buffer.channel.topic.enabled, |settings| {
            settings.channel.topic.enabled
        });

    if !topic_enabled {
        return None;
    }

    let casemapping = clients.get_casemapping(&state.server);

    let topic = clients.get_channel_topic(&state.server, &state.target)?;

    Some(
        topic::view(
            &state.server,
            casemapping,
            &state.target,
            topic.content.as_ref()?,
            topic.who.as_deref(),
            topic.time.as_ref(),
            config.buffer.channel.topic.max_lines,
            users,
            our_user,
            config,
            theme,
        )
        .map(Message::Topic),
    )
}

mod nick_list {
    use data::{Config, Server, User, config, isupport, target};
    use iced::Length;
    use iced::advanced::text;
    use iced::widget::{Scrollable, column, scrollable};
    use user_context::Message;

    use crate::buffer::user_context;
    use crate::widget::{Element, selectable_text};
    use crate::{font, theme};

    pub fn view<'a>(
        server: &'a Server,
        casemapping: isupport::CaseMap,
        channel: &'a target::Channel,
        users: &'a [User],
        our_user: Option<&'a User>,
        config: &'a Config,
    ) -> Element<'a, Message> {
        let nicklist_config = &config.buffer.channel.nicklist;

        let width = match nicklist_config.width {
            Some(width) => width,
            None => {
                let max_nick_length = users
                    .iter()
                    .map(|user| {
                        user.display(nicklist_config.show_access_levels)
                            .chars()
                            .count()
                    })
                    .max()
                    .unwrap_or_default();

                font::width_from_chars(max_nick_length, &config.font)
            }
        };

        let content = column(users.iter().map(|user| {
            let content = selectable_text(
                user.display(nicklist_config.show_access_levels),
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

            user_context::view(
                content,
                server,
                casemapping,
                Some(channel),
                user,
                Some(user),
                our_user,
                config,
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
