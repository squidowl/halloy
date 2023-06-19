use data::server::Server;
use data::{history, User};
use iced::widget::{button, column, container, horizontal_space, pane_grid, row, text};
use iced::Length;

use super::pane::Pane;
use crate::widget::{context_menu, Element};
use crate::{buffer, icon, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Channel(Server, String),
    Server(Server),
    Query(Server, User),
}

#[derive(Debug, Clone)]
pub enum Event {
    Channel(Server, String),
    Server(Server),
    Query(Server, User),
}

#[derive(Clone)]
pub struct SideMenu {}

impl SideMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::Channel(server, channel) => Some(Event::Channel(server, channel)),
            Message::Server(server) => Some(Event::Server(server)),
            Message::Query(server, user) => Some(Event::Query(server, user)),
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
            column = column.push(source_button(panes, focus, Source::Server(server.clone())));

            for channel in channels {
                column = column.push(source_button(
                    panes,
                    focus,
                    Source::Channel(server.clone(), channel.clone()),
                ));
            }

            let queries = history.get_unique_queries(server);
            for user in queries {
                column = column.push(source_button(
                    panes,
                    focus,
                    Source::Query(server.clone(), user.clone()),
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
    fn list(open: Option<pane_grid::Pane>, focus: Option<pane_grid::Pane>) -> Vec<Self> {
        match (open, focus) {
            (None, None) => vec![Entry::NewPane],
            (None, Some(focus)) => vec![Entry::NewPane, Entry::Replace(focus)],
            (Some(open), None) => vec![Entry::Close(open)],
            (Some(open), Some(focus)) => vec![Entry::Close(open), Entry::Swap(open, focus)],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Source {
    Server(Server),
    Channel(Server, String),
    Query(Server, User),
}

fn source_button<'a>(
    panes: &pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    source: Source,
) -> Element<'a, Message> {
    let open = panes
        .iter()
        .find_map(|(pane, state)| match (&state.buffer, &source) {
            (buffer::Buffer::Server(state), Source::Server(server)) => {
                (&state.server == server).then_some(*pane)
            }
            (buffer::Buffer::Channel(state), Source::Channel(server, channel)) => {
                (&state.server == server && &state.channel == channel).then_some(*pane)
            }
            (buffer::Buffer::Query(state), Source::Query(server, user)) => {
                (&state.server == server && &state.user == user).then_some(*pane)
            }
            _ => None,
        });

    let row = match &source {
        Source::Server(server) => row![icon::globe(), text(server.to_string())]
            .spacing(8)
            .align_items(iced::Alignment::Center),
        Source::Channel(_, channel) => row![horizontal_space(4), icon::chat(), text(channel)]
            .spacing(8)
            .align_items(iced::Alignment::Center),
        Source::Query(_, user) => row![horizontal_space(4), icon::person(), text(user.nickname())]
            .spacing(8)
            .align_items(iced::Alignment::Center),
    };

    let base = button(row)
        .width(Length::Fill)
        .style(theme::Button::SideMenu {
            selected: open.is_some(),
        })
        .on_press(match &source {
            Source::Server(server) => Message::Server(server.clone()),
            Source::Channel(server, channel) => Message::Channel(server.clone(), channel.clone()),
            Source::Query(server, user) => Message::Query(server.clone(), user.clone()),
        });

    let entries = Entry::list(open, focus);

    context_menu(base, entries, move |entry, hovered| {
        // TODO: Different messages per action
        let message = match &source {
            Source::Server(server) => Message::Server(server.clone()),
            Source::Channel(server, channel) => Message::Channel(server.clone(), channel.clone()),
            Source::Query(server, user) => Message::Query(server.clone(), user.clone()),
        };

        let (content, message) = match entry {
            Entry::NewPane => ("Open in new pane", message),
            Entry::Replace(_pane) => ("Replace current pane", message),
            Entry::Close(_pane) => ("Close pane", message),
            Entry::Swap(_a, _b) => ("Swap with current pane", message),
        };

        button(text(content))
            // Based off longest entry text
            .width(175)
            // TODO: Better styling
            .style(theme::Button::SideMenu { selected: hovered })
            .on_press(message)
            .into()
    })
    .into()
}
