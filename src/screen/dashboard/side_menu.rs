use data::dashboard::DefaultAction;
use data::{history, Buffer};
use iced::widget::text::LineHeight;
use iced::widget::{
    button, column, container, horizontal_space, pane_grid, row, scrollable, text, vertical_space,
};
use iced::Length;

use super::pane::Pane;
use crate::widget::{context_menu, Collection, Element};
use crate::{icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Open(Buffer),
    Replace(Buffer, pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
    Leave(Buffer),
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(Buffer),
    Replace(Buffer, pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
    Leave(Buffer),
}

#[derive(Clone)]
pub struct SideMenu {
    hidden: bool,
}

impl SideMenu {
    pub fn new() -> Self {
        Self { hidden: false }
    }

    pub fn toggle_visibility(&mut self) {
        self.hidden = !self.hidden
    }

    pub fn update(&mut self, message: Message) -> Event {
        match message {
            Message::Open(source) => Event::Open(source),
            Message::Replace(source, pane) => Event::Replace(source, pane),
            Message::Close(pane) => Event::Close(pane),
            Message::Swap(from, to) => Event::Swap(from, to),
            Message::Leave(buffer) => Event::Leave(buffer),
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        history: &'a history::Manager,
        panes: &pane_grid::State<Pane>,
        focus: Option<pane_grid::Pane>,
        config: data::config::dashboard::Sidebar,
        font: &data::config::Font,
    ) -> Option<Element<'a, Message>> {
        if self.hidden {
            return None;
        }

        let mut column = column![].spacing(1);

        for (server, state) in clients.iter() {
            match state {
                data::client::State::Disconnected => {
                    column = column.push(buffer_button(
                        panes,
                        focus,
                        Buffer::Server(server.clone()),
                        false,
                        false,
                        config.default_action,
                        font,
                    ));
                }
                data::client::State::Ready(connection) => {
                    column = column.push(buffer_button(
                        panes,
                        focus,
                        Buffer::Server(server.clone()),
                        true,
                        false,
                        config.default_action,
                        font,
                    ));

                    for channel in connection.channels() {
                        column = column.push(buffer_button(
                            panes,
                            focus,
                            Buffer::Channel(server.clone(), channel.clone()),
                            true,
                            history.has_unread(server, &history::Kind::Channel(channel.clone())),
                            config.default_action,
                            font,
                        ));
                    }

                    let queries = history.get_unique_queries(server);
                    for user in queries {
                        column = column.push(buffer_button(
                            panes,
                            focus,
                            Buffer::Query(server.clone(), user.clone()),
                            true,
                            history.has_unread(server, &history::Kind::Query(user.clone())),
                            config.default_action,
                            font,
                        ));
                    }

                    column = column.push(vertical_space(12));
                }
            }
        }

        Some(
            container(
                scrollable(column).direction(scrollable::Direction::Vertical(
                    iced::widget::scrollable::Properties::default()
                        .width(0)
                        .scroller_width(0),
                )),
            )
            .padding([8, 0, 6, 6])
            .center_x()
            .max_width(config.width)
            .into(),
        )
    }
}

#[derive(Debug, Clone, Copy)]
enum Entry {
    NewPane,
    Replace(pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
    Leave,
}

impl Entry {
    fn list(
        num_panes: usize,
        open: Option<pane_grid::Pane>,
        focus: Option<pane_grid::Pane>,
    ) -> Vec<Self> {
        match (open, focus) {
            (None, None) => vec![Entry::NewPane, Entry::Leave],
            (None, Some(focus)) => vec![Entry::NewPane, Entry::Replace(focus), Entry::Leave],
            (Some(open), None) => (num_panes > 1)
                .then_some(Entry::Close(open))
                .into_iter()
                .chain(Some(Entry::Leave))
                .collect(),
            (Some(open), Some(focus)) => (num_panes > 1)
                .then_some(Entry::Close(open))
                .into_iter()
                .chain((open != focus).then_some(Entry::Swap(open, focus)))
                .chain(Some(Entry::Leave))
                .collect(),
        }
    }
}

fn buffer_button<'a>(
    panes: &pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    buffer: Buffer,
    connected: bool,
    has_unread: bool,
    default_action: DefaultAction,
    font: &data::config::Font,
) -> Element<'a, Message> {
    let font = font.clone();
    let open = panes
        .iter()
        .find_map(|(pane, state)| (state.buffer.data().as_ref() == Some(&buffer)).then_some(*pane));

    let row = match &buffer {
        Buffer::Server(server) => row![
            if connected {
                icon::globe()
            } else {
                icon::wifi_off()
            },
            text(server.to_string()).style(theme::Text::Primary)
        ]
        .spacing(8)
        .align_items(iced::Alignment::Center),
        Buffer::Channel(_, channel) => row![]
            .push(horizontal_space(3))
            .push_maybe(has_unread.then_some(icon::dot().size(6).style(theme::Text::Info)))
            .push(horizontal_space(if has_unread { 10 } else { 16 }))
            .push(text(channel).style(theme::Text::Primary))
            .align_items(iced::Alignment::Center),
        Buffer::Query(_, nick) => row![]
            .push(horizontal_space(3))
            .push_maybe(has_unread.then_some(icon::dot().size(6).style(theme::Text::Info)))
            .push(horizontal_space(if has_unread { 10 } else { 16 }))
            .push(text(nick).style(theme::Text::Primary))
            .align_items(iced::Alignment::Center),
    };

    let base = button(row)
        .width(Length::Fill)
        .style(theme::Button::SideMenu {
            selected: open.is_some(),
        })
        .on_press(match default_action {
            DefaultAction::NewPane => Message::Open(buffer.clone()),
            DefaultAction::ReplacePane => match focus {
                Some(pane) => Message::Replace(buffer.clone(), pane),
                None => Message::Open(buffer.clone()),
            },
        });

    let entries = Entry::list(panes.len(), open, focus);

    if entries.is_empty() || !connected {
        base.into()
    } else {
        context_menu(base, entries, move |entry, length| {
            let (content, message) = match entry {
                Entry::NewPane => ("Open in new pane", Message::Open(buffer.clone())),
                Entry::Replace(pane) => (
                    "Replace current pane",
                    Message::Replace(buffer.clone(), pane),
                ),
                Entry::Close(pane) => ("Close pane", Message::Close(pane)),
                Entry::Swap(from, to) => ("Swap with current pane", Message::Swap(from, to)),
                Entry::Leave => (
                    match &buffer {
                        Buffer::Server(_) => "Leave server",
                        Buffer::Channel(_, _) => "Leave channel",
                        Buffer::Query(_, _) => "Close query",
                    },
                    Message::Leave(buffer.clone()),
                ),
            };

            // Height based on font size, with some margin added.
            let height = LineHeight::default().to_absolute((font.size + 8.0).into());

            button(text(content).style(theme::Text::Primary))
                .width(length)
                .height(height)
                .style(theme::Button::Context)
                .on_press(message)
                .into()
        })
    }
}
