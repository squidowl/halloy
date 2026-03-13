use std::borrow::Cow;
use std::collections::HashSet;

use data::{Config, message};
use iced::Length;
use iced::widget::{
    Scrollable, button, column, container, scrollable, text_input,
};

use crate::emoji;
use crate::widget::{Element, Row, text};
use crate::{theme, widget};

const MODAL_WIDTH: f32 = 380.0;
const MODAL_HEIGHT: f32 = 250.0;
const EMOJI_BUTTON_WIDTH: f32 = 32.0;
const EMOJI_BUTTON_HEIGHT: f32 = 32.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    msgid: message::Id,
    selected_reactions: HashSet<Cow<'static, str>>,
    search_query: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
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
    pub fn new(msgid: message::Id, selected_reactions: Vec<String>) -> Self {
        Self {
            msgid,
            selected_reactions: selected_reactions
                .into_iter()
                .map(Cow::Owned)
                .collect(),
            search_query: String::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::SearchChanged(search_query) => {
                self.search_query = search_query;
                None
            }
            Message::SelectEmoji(text) => {
                let unreact = self.selected_reactions.contains(&text);

                Some(Event::Toggle {
                    msgid: self.msgid.clone(),
                    text,
                    unreact,
                })
            }
        }
    }
}

pub fn view<'a>(state: &'a State, config: &'a Config) -> Element<'a, Message> {
    let query = normalized_query(&state.search_query);
    let filtered = filtered_emojis(&query);
    let has_results = !filtered.is_empty();
    let grid = emoji_grid(&filtered, &state.selected_reactions);

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
        text_input("Search..", &state.search_query)
            .on_input(Message::SearchChanged)
            .padding(8),
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
    selected_reactions: &HashSet<Cow<'static, str>>,
) -> Element<'a, Message> {
    emojis
        .iter()
        .fold(Row::new().spacing(4), |row, emoji| {
            row.push(emoji_button(
                emoji,
                selected_reactions.contains(emoji.as_str()),
            ))
        })
        .wrap()
        .into()
}

fn emoji_button<'a>(
    emoji: &'static emojis::Emoji,
    selected: bool,
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
        theme::button::secondary(theme, status, selected)
    })
    .on_press(Message::SelectEmoji(Cow::Borrowed(emoji.as_str())))
}

fn filtered_emojis(query: &str) -> Vec<&'static emojis::Emoji> {
    emoji::matching_emojis(query)
}

fn normalized_query(query: &str) -> String {
    query.trim().trim_matches(':').to_ascii_lowercase()
}
