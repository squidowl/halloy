use chrono::{DateTime, Utc};
use data::message::Source;
use data::{Config, User, history, message, target};
use iced::widget::{
    self, button, column, container, operation, row, scrollable, text,
    text_input,
};
use iced::{Length, Size, Task, alignment, padding};

use crate::widget::key_press::{Key, Modifiers, Named};
use crate::widget::selectable_text;
use crate::widget::{Element, key_press};
use crate::{Theme, font, theme};

const MAX_RESULTS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub server: data::Server,
    pub channel: target::Channel,
}

impl Scope {
    pub fn new(server: data::Server, channel: target::Channel) -> Self {
        Self { server, channel }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    Submit,
    SelectResult(usize),
}

pub enum Event {
    GoToMessage(data::Server, target::Channel, message::Hash),
}

#[derive(Debug, Clone)]
struct ResultRow {
    hash: message::Hash,
    timestamp: DateTime<Utc>,
    sender: Option<User>,
    text: String,
}

#[derive(Debug, Clone)]
pub struct Search {
    scope: Option<Scope>,
    query: String,
    results: Vec<ResultRow>,
    query_id: widget::Id,
}

impl Search {
    pub fn new(
        scope: Option<Scope>,
        _pane_size: Size,
        _config: &Config,
    ) -> Self {
        Self {
            scope,
            query: String::new(),
            results: vec![],
            query_id: widget::Id::unique(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        history: &history::Manager,
        config: &Config,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::QueryChanged(query) => {
                self.query = query;
                (Task::none(), None)
            }
            Message::Submit => {
                self.search(history, config);
                (Task::none(), None)
            }
            Message::SelectResult(index) => {
                let event = self.results.get(index).and_then(|result| {
                    self.scope.as_ref().map(|scope| {
                        Event::GoToMessage(
                            scope.server.clone(),
                            scope.channel.clone(),
                            result.hash,
                        )
                    })
                });

                (Task::none(), event)
            }
        }
    }

    pub fn focus(&self) -> Task<Message> {
        let query_id = self.query_id.clone();

        operation::is_focused(query_id.clone()).then(move |is_focused| {
            if is_focused {
                Task::none()
            } else {
                operation::focus(query_id.clone())
            }
        })
    }

    pub fn reset(&mut self) {
        self.query.clear();
        self.results.clear();
    }

    fn search(&mut self, history: &history::Manager, config: &Config) {
        let Some(scope) = self.scope.as_ref() else {
            self.results.clear();
            return;
        };

        let query = self.query.trim();

        if query.is_empty() {
            self.results.clear();
            return;
        }

        let query = query.to_lowercase();
        let kind = history::Kind::Channel(
            scope.server.clone(),
            scope.channel.clone(),
        );

        let Some(view) = history.get_messages(&kind, None, config) else {
            self.results.clear();
            return;
        };

        self.results = view
            .old_messages
            .iter()
            .chain(view.new_messages.iter())
            .rev()
            .filter_map(|message| {
                let text = message.text();

                let sender = match message.target.source() {
                    Source::User(user) => Some(user.clone()),
                    Source::Action(Some(user)) => Some(user.clone()),
                    _ => None,
                };

                text.to_lowercase().contains(&query).then_some(ResultRow {
                    hash: message.hash,
                    timestamp: message.server_time,
                    sender,
                    text,
                })
            })
            .take(MAX_RESULTS)
            .collect();
    }
}

pub fn view<'a>(
    state: &'a Search,
    _history: &'a history::Manager,
    config: &'a Config,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let heading = match state.scope.as_ref() {
        Some(scope) => {
            text(format!("Searching in {}", scope.channel)).font_maybe(
                theme::font_style::primary(theme).map(font::get),
            )
        }
        None => text("Search is only available from a channel buffer")
            .style(theme::text::secondary)
            .font_maybe(theme::font_style::secondary(theme).map(font::get)),
    };

    let input = text_input("Search term", &state.query)
        .id(state.query_id.clone())
        .style(move |theme, status| {
            if matches!(status, text_input::Status::Disabled) {
                theme::text_input::primary(theme, text_input::Status::Active)
            } else {
                theme::text_input::primary(theme, status)
            }
        })
        .on_input_maybe(
            state
                .scope
                .is_some()
                .then_some(Message::QueryChanged),
        );

    let input = key_press(
        input,
        Key::Named(Named::Enter),
        Modifiers::default(),
        Message::Submit,
    );

    let controls = row![
        container(input).width(Length::Fill),
        button(text("Search")).on_press_maybe(
            state.scope.is_some().then_some(Message::Submit)
        )
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    let results: Element<'_, Message> = if state.query.trim().is_empty() {
        container(
            text("Enter a search term and press Enter")
                .style(theme::text::secondary)
                .font_maybe(theme::font_style::secondary(theme).map(font::get)),
        )
        .padding([8, 4])
        .into()
    } else if state.results.is_empty() {
        container(
            text("No results found")
                .style(theme::text::secondary)
                .font_maybe(theme::font_style::secondary(theme).map(font::get)),
        )
        .padding([8, 4])
        .into()
    } else {
        search_results_view(state, config, theme)
        .into()
    };

    container(
        column![heading, controls, results]
            .spacing(8)
            .padding(padding::top(8)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(8)
    .into()
}

fn search_results_view<'a>(state: &'a Search, config: &'a Config, theme: &'a Theme) -> widget::Scrollable<'a, Message, Theme> {
    scrollable(column(
        state
            .results
            .iter()
            .enumerate()
            .map(|(index, result)| {
                let timestamp = config
                    .buffer
                    .format_timestamp(&result.timestamp)
                    .unwrap_or_default();

                let mut row_items: Vec<Element<'_, Message>> =
                    Vec::with_capacity(3);

                if !timestamp.is_empty() {
                    row_items.push(
                        selectable_text(timestamp)
                            .style(theme::selectable_text::timestamp)
                            .font_maybe(
                                theme::font_style::timestamp(theme)
                                    .map(font::get),
                            )
                            .into(),
                    );
                }

                if let Some(user) = result.sender.as_ref() {
                    let nick_style =
                        theme::selectable_text::nickname(
                            theme, config, user, false,
                        );
                    let brackets = &config.buffer.nickname.brackets;
                    let nick_str = brackets.format(user.nickname().as_str());
                    row_items.push(
                        selectable_text(nick_str)
                            .style(move |_| nick_style)
                            .font_maybe(
                                theme::font_style::nickname(
                                    theme, false,
                                )
                                .map(font::get),
                            )
                            .into(),
                    );
                }

                row_items.push(
                    selectable_text(result.text.as_str())
                        .style(|theme: &Theme| crate::widget::selectable_text::Style {
                            color: Some(theme.styles().text.primary.color),
                            selection_color: theme.styles().buffer.selection,
                        })
                        .font_maybe(
                            theme::font_style::primary(theme)
                                .map(font::get),
                        )
                        .into(),
                );

                button(
                    row(row_items)
                        .spacing(8)
                        .width(Length::Fill),
                )
                .style(theme::button::bare)
                .on_press(Message::SelectResult(index))
                .width(Length::Fill)
                .padding([4, 0])
                .into()
            }),
    ))
}
