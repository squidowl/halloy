use std::borrow::Cow;
use std::collections::HashSet;

use data::{Config, message};
use iced::widget::{
    Scrollable, button, column, container, operation, scrollable, text_input,
};
use iced::{Length, Task};

use crate::widget::{Element, Row, key_press, text};
use crate::{emoji, theme, widget};

const MODAL_WIDTH: f32 = 380.0;
const MODAL_HEIGHT: f32 = 250.0;
const EMOJI_BUTTON_WIDTH: f32 = 32.0;
const EMOJI_BUTTON_HEIGHT: f32 = 32.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    msgid: message::Id,
    already_reacted: HashSet<Cow<'static, str>>,
    search_query_id: iced::widget::Id,
    search_query: String,
    selection: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    Tab(bool),
    SearchSelect,
    SelectEmoji(Cow<'static, str>),
}

#[derive(Debug, Clone)]
pub enum Event {
    Toggle {
        msgid: message::Id,
        text: Cow<'static, str>,
        unreact: bool,
    },
}

impl State {
    pub fn new(msgid: message::Id, already_reacted: Vec<String>) -> Self {
        Self {
            msgid,
            already_reacted: already_reacted
                .into_iter()
                .map(Cow::Owned)
                .collect(),
            search_query_id: iced::widget::Id::unique(),
            search_query: String::new(),
            selection: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::SearchChanged(search_query) => {
                self.search_query = search_query;

                let filtered = self.filtered_emojis();

                if filtered.is_empty() {
                    self.selection = None;
                } else {
                    self.selection = Some(0);
                }

                None
            }
            Message::Tab(shift) => {
                let filtered = self.filtered_emojis();

                if filtered.is_empty() {
                    self.selection = None;
                } else {
                    self.selection =
                        Some(if let Some(selection) = self.selection {
                            if shift {
                                if selection == 0 {
                                    filtered.len() - 1
                                } else {
                                    selection - 1
                                }
                            } else {
                                (selection + 1) % filtered.len()
                            }
                        } else if shift {
                            filtered.len() - 1
                        } else {
                            0
                        });
                }

                None
            }
            Message::SearchSelect => self.selection.and_then(|selection| {
                self.filtered_emojis().get(selection).map(|emoji| {
                    let unreact = self.already_reacted.contains(emoji.as_str());

                    Event::Toggle {
                        msgid: self.msgid.clone(),
                        text: Cow::Borrowed(emoji.as_str()),
                        unreact,
                    }
                })
            }),
            Message::SelectEmoji(text) => {
                let unreact = self.already_reacted.contains(&text);

                Some(Event::Toggle {
                    msgid: self.msgid.clone(),
                    text,
                    unreact,
                })
            }
        }
    }

    pub fn focus(&self) -> Task<Message> {
        let search_query_id = self.search_query_id.clone();

        operation::is_focused(search_query_id.clone()).then(move |is_focused| {
            if is_focused {
                Task::none()
            } else {
                operation::focus(search_query_id.clone())
            }
        })
    }

    fn filtered_emojis(&self) -> Vec<&'static emojis::Emoji> {
        let query = normalized_query(&self.search_query);

        emoji::matching_emojis(&query)
    }
}

pub fn view<'a>(state: &'a State, config: &'a Config) -> Element<'a, Message> {
    let filtered = state.filtered_emojis();
    let has_results = !filtered.is_empty();
    let grid = emoji_grid(&filtered, &state.already_reacted, state.selection);

    let body: Element<'a, Message> = if has_results {
        Scrollable::new(container(grid).width(Length::Fill))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::default()
                    .width(config.pane.scrollbar.width)
                    .scroller_width(config.pane.scrollbar.scroller_width),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        container(text("No emoji matches"))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    };

    let content = column![
        key_press(
            key_press(
                text_input("Search...", &state.search_query)
                    .id(state.search_query_id.clone())
                    .on_input(Message::SearchChanged)
                    .on_submit(Message::SearchSelect)
                    .padding(8),
                key_press::Key::Named(key_press::Named::Tab),
                key_press::Modifiers::SHIFT,
                Message::Tab(true),
            ),
            key_press::Key::Named(key_press::Named::Tab),
            key_press::Modifiers::default(),
            Message::Tab(false),
        ),
        body
    ]
    .spacing(12)
    .height(Length::Fill);

    container(content)
        .width(Length::Fixed(MODAL_WIDTH))
        .height(Length::Fixed(MODAL_HEIGHT))
        .padding(8)
        .style(theme::container::tooltip)
        .into()
}

fn emoji_grid<'a>(
    emojis: &[&'static emojis::Emoji],
    already_reacted: &HashSet<Cow<'static, str>>,
    selection: Option<usize>,
) -> Element<'a, Message> {
    emojis
        .iter()
        .enumerate()
        .fold(Row::new().spacing(4), |row, (index, emoji)| {
            row.push(emoji_button(
                emoji,
                already_reacted.contains(emoji.as_str()),
                selection.is_some_and(|selection| selection == index),
            ))
        })
        .wrap()
        .into()
}

fn emoji_button<'a>(
    emoji: &'static emojis::Emoji,
    already_reacted: bool,
    selection: bool,
) -> widget::Button<'a, Message> {
    button(
        container(text(emoji.as_str()).size(16))
            .width(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .padding(4)
    .width(Length::Fixed(EMOJI_BUTTON_WIDTH))
    .height(Length::Fixed(EMOJI_BUTTON_HEIGHT))
    .style(move |theme, status| {
        theme::button::reaction(theme, status, already_reacted, selection)
    })
    .on_press(Message::SelectEmoji(Cow::Borrowed(emoji.as_str())))
}

fn normalized_query(query: &str) -> String {
    query.trim().trim_matches(':').to_ascii_lowercase()
}
