use data::{history, Buffer};
use iced::widget::{button, column, container, horizontal_space, pane_grid, row, text};
use iced::Length;

use super::pane::Pane;
use crate::widget::{context_menu, Element};
use crate::{icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Open(Buffer),
    Replace(Buffer, pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(Buffer),
    Replace(Buffer, pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
}

#[derive(Clone)]
pub struct SideMenu {}

impl SideMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::Open(source) => Some(Event::Open(source)),
            Message::Replace(source, pane) => Some(Event::Replace(source, pane)),
            Message::Close(pane) => Some(Event::Close(pane)),
            Message::Swap(from, to) => Some(Event::Swap(from, to)),
        }
    }

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        history: &'a history::Manager,
        panes: &pane_grid::State<Pane>,
        focus: Option<pane_grid::Pane>,
    ) -> Element<'a, Message> {
        let mut column = column![].spacing(1);

        for (server, channels) in clients.get_channels().iter() {
            column = column.push(buffer_button(panes, focus, Buffer::Server(server.clone())));

            for channel in channels {
                column = column.push(buffer_button(
                    panes,
                    focus,
                    Buffer::Channel(server.clone(), channel.clone()),
                ));
            }

            let queries = history.get_unique_queries(server);
            for user in queries {
                column = column.push(buffer_button(
                    panes,
                    focus,
                    Buffer::Query(server.clone(), user.clone()),
                ));
            }
        }

        container(column)
            .padding([8, 0, 6, 6])
            .center_x()
            .max_width(120)
            .into()
    }
}

#[derive(Debug, Clone, Copy)]
enum Entry {
    NewPane,
    Replace(pane_grid::Pane),
    Close(pane_grid::Pane),
    Swap(pane_grid::Pane, pane_grid::Pane),
}

impl Entry {
    fn list(
        num_panes: usize,
        open: Option<pane_grid::Pane>,
        focus: Option<pane_grid::Pane>,
    ) -> Vec<Self> {
        match (open, focus) {
            (None, None) => vec![Entry::NewPane],
            (None, Some(focus)) => vec![Entry::NewPane, Entry::Replace(focus)],
            (Some(open), None) => (num_panes > 1)
                .then_some(Entry::Close(open))
                .into_iter()
                .collect(),
            (Some(open), Some(focus)) => (num_panes > 1)
                .then_some(Entry::Close(open))
                .into_iter()
                .chain((open != focus).then_some(Entry::Swap(open, focus)))
                .collect(),
        }
    }
}

fn buffer_button<'a>(
    panes: &pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    buffer: Buffer,
) -> Element<'a, Message> {
    let open = panes
        .iter()
        .find_map(|(pane, state)| (state.buffer.kind().as_ref() == Some(&buffer)).then_some(*pane));

    let row = match &buffer {
        Buffer::Server(server) => row![icon::globe(), text(server.to_string())]
            .spacing(8)
            .align_items(iced::Alignment::Center),
        Buffer::Channel(_, channel) => row![horizontal_space(4), icon::chat(), text(channel)]
            .spacing(8)
            .align_items(iced::Alignment::Center),
        Buffer::Query(_, user) => row![horizontal_space(4), icon::person(), text(user.nickname())]
            .spacing(8)
            .align_items(iced::Alignment::Center),
    };

    let base = button(row)
        .width(Length::Fill)
        .style(theme::Button::SideMenu {
            selected: open.is_some(),
        })
        .on_press(Message::Open(buffer.clone()));

    let entries = Entry::list(panes.len(), open, focus);

    if entries.is_empty() {
        base.into()
    } else {
        context_menu(base, entries, move |entry| {
            let (content, message) = match entry {
                Entry::NewPane => ("Open in new pane", Message::Open(buffer.clone())),
                Entry::Replace(pane) => (
                    "Replace current pane",
                    Message::Replace(buffer.clone(), pane),
                ),
                Entry::Close(pane) => ("Close pane", Message::Close(pane)),
                Entry::Swap(from, to) => ("Swap with current pane", Message::Swap(from, to)),
            };

            button(text(content))
                // Based off longest entry text
                .width(175)
                .style(theme::Button::Context)
                .on_press(message)
                .into()
        })
    }
}
