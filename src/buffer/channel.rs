use data::server::Server;
use data::{channel, client, history, message, Config};
use iced::widget::{column, container, row, vertical_space};
use iced::{Command, Length};

use super::{input_view, scroll_view, user_context};
use crate::theme;
use crate::widget::{selectable_text, Collection, Element};

#[derive(Debug, Clone)]
pub enum Message {
    ScrollView(scroll_view::Message),
    InputView(input_view::Message),
    UserContext(user_context::Message),
}

#[derive(Debug, Clone)]
pub enum Event {
    UserContext(user_context::Event),
}

pub fn view<'a>(
    state: &'a Channel,
    status: client::Status,
    clients: &'a data::client::Map,
    history: &'a history::Manager,
    settings: &'a channel::Settings,
    config: &'a Config,
    is_focused: bool,
) -> Element<'a, Message> {
    let buffer = state.buffer();
    let input_history = history.input_history(&buffer);
    let our_nick = clients.nickname(&state.server);

    let messages = container(
        scroll_view::view(
            &state.scroll_view,
            scroll_view::Kind::Channel(&state.server, &state.channel),
            history,
            config,
            move |message| {
                let timestamp = config
                    .buffer
                    .format_timestamp(&message.server_time)
                    .map(|timestamp| selectable_text(timestamp).style(theme::Text::Transparent));

                match message.target.source() {
                    message::Source::User(user) => {
                        let nick = user_context::view(
                            selectable_text(config.buffer.nickname.brackets.format(user)).style(
                                theme::Text::Nickname(
                                    user.color_seed(&config.buffer.nickname.color),
                                ),
                            ),
                            user.clone(),
                        )
                        .map(scroll_view::Message::UserContext);
                        let row_style = match our_nick {
                            Some(nick)
                                if message::reference_user(
                                    user.nickname(),
                                    nick,
                                    &message.text,
                                ) =>
                            {
                                theme::Container::Highlight
                            }
                            _ => theme::Container::Default,
                        };
                        let message = selectable_text(&message.text);

                        Some(
                            container(row![].push_maybe(timestamp).push(nick).push(message))
                                .style(row_style)
                                .into(),
                        )
                    }
                    message::Source::Server(_) => {
                        let message = selectable_text(&message.text).style(theme::Text::Server);

                        Some(container(row![].push_maybe(timestamp).push(message)).into())
                    }
                    message::Source::Action => {
                        let message = selectable_text(&message.text).style(theme::Text::Accent);

                        Some(container(row![].push_maybe(timestamp).push(message)).into())
                    }
                    message::Source::Internal(message::source::Internal::Status(status)) => {
                        let message =
                            selectable_text(&message.text).style(theme::Text::Status(*status));

                        Some(container(row![].push_maybe(timestamp).push(message)).into())
                    }
                }
            },
        )
        .map(Message::ScrollView),
    )
    .width(Length::FillPortion(2))
    .height(Length::Fill);

    let users = clients.get_channel_users(&state.server, &state.channel);
    let channels = clients.get_channels(&state.server);
    let nick_list = nick_list::view(users).map(Message::UserContext);

    let show_text_input = match config.buffer.input_visibility {
        data::buffer::InputVisibility::Focused => is_focused && status.connected(),
        data::buffer::InputVisibility::Always => status.connected(),
    };

    let text_input = show_text_input.then(|| {
        column![
            vertical_space(4),
            input_view::view(
                &state.input_view,
                buffer,
                users,
                channels,
                input_history,
                is_focused
            )
            .map(Message::InputView)
        ]
    });

    let content = match (settings.users.visible, config.buffer.channel.users.position) {
        (true, data::channel::Position::Left) => {
            row![nick_list, messages]
        }
        (true, data::channel::Position::Right) => {
            row![messages, nick_list]
        }
        (false, _) => { row![messages] }.height(Length::Fill),
    };

    let scrollable = column![container(content).height(Length::Fill)]
        .push_maybe(text_input)
        .height(Length::Fill);

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .into()
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub server: Server,
    pub channel: String,
    pub topic: Option<String>,
    pub scroll_view: scroll_view::State,
    pub input_view: input_view::State,
}

impl Channel {
    pub fn new(server: Server, channel: String) -> Self {
        Self {
            server,
            channel,
            topic: None,
            scroll_view: scroll_view::State::new(),
            input_view: input_view::State::new(),
        }
    }

    pub fn buffer(&self) -> data::Buffer {
        data::Buffer::Channel(self.server.clone(), self.channel.clone())
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        history: &mut history::Manager,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::ScrollView(message) => {
                let (command, event) = self.scroll_view.update(message);

                let event = event.map(|event| match event {
                    scroll_view::Event::UserContext(event) => Event::UserContext(event),
                });

                (command.map(Message::ScrollView), event)
            }
            Message::InputView(message) => {
                let (command, event) = self.input_view.update(message, clients, history);
                let command = command.map(Message::InputView);

                match event {
                    Some(input_view::Event::InputSent) => {
                        let command = Command::batch(vec![
                            command,
                            self.scroll_view.scroll_to_end().map(Message::ScrollView),
                        ]);

                        (command, None)
                    }
                    None => (command, None),
                }
            }
            Message::UserContext(message) => (
                Command::none(),
                Some(Event::UserContext(user_context::update(message))),
            ),
        }
    }

    pub fn focus(&self) -> Command<Message> {
        self.input_view.focus().map(Message::InputView)
    }

    pub fn reset(&self) -> Command<Message> {
        self.input_view.reset().map(Message::InputView)
    }
}

mod nick_list {
    use data::User;
    use iced::widget::{column, container, scrollable, text};
    use iced::Length;
    use user_context::Message;

    use crate::buffer::user_context;
    use crate::theme;
    use crate::widget::Element;

    pub fn view(users: &[User]) -> Element<Message> {
        let column = column(users.iter().map(|user| {
            let content = text(format!(
                "{}{}",
                user.highest_access_level(),
                user.nickname()
            ))
            .style(if user.is_away() {
                theme::Text::Transparent
            } else {
                theme::Text::Primary
            });

            user_context::view(content, user.clone())
        }))
        .padding(4)
        .spacing(1);

        container(
            scrollable(column)
                .direction(scrollable::Direction::Vertical(
                    scrollable::Properties::new().width(1).scroller_width(1),
                ))
                .style(theme::Scrollable::Hidden),
        )
        .width(Length::Shrink)
        .max_width(120)
        .height(Length::Fill)
        .into()
    }
}
