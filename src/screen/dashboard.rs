pub mod pane;

use pane::Pane;

use iced::pure::widget::pane_grid::{self, PaneGrid};
use iced::pure::Element;
use iced::pure::{column, container};
use iced::Command;
use iced::Length;

use crate::buffer::{self, Buffer};
use crate::config::Config;
use crate::style;
use crate::theme::Theme;

pub struct Dashboard {
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Pane(pane::Message),
    Buffer(pane_grid::Pane, buffer::Message),
    PaneClicked(pane_grid::Pane),
    PaneResized(pane_grid::ResizeEvent),
    PaneDragged(pane_grid::DragEvent),
    ClosePane,
    SplitPane(pane_grid::Axis),
}

pub enum Event {}

impl Dashboard {
    pub fn new(config: &Config) -> Self {
        let (mut panes, pane) = pane_grid::State::new(Pane::new(Buffer::Empty));

        // TODO: Create initial panels (channels) more nicely.
        for server in config.servers.iter() {
            for channel in server.channels() {
                panes.split(
                    pane_grid::Axis::Horizontal,
                    &pane,
                    Pane::new(Buffer::Channel(
                        server.server.clone().unwrap().into(),
                        channel.as_str().parse().unwrap(),
                    )),
                );
            }
        }

        // TODO: A little hacke for now, just to get the ball rolling.
        if config.servers.len() > 0 {
            panes.close(&pane);
        }

        Dashboard { panes, focus: None }
    }

    pub fn update(&mut self, message: Message) -> Option<(Event, Command<Message>)> {
        match message {
            Message::PaneClicked(pane) => {
                self.focus = Some(pane);
            }
            Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);
            }
            Message::PaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.panes.swap(&pane, &target);
            }
            Message::PaneDragged(_) => {}
            Message::ClosePane => {
                if let Some(pane) = self.focus {
                    if let Some((_, sibling)) = self.panes.close(&pane) {
                        self.focus = Some(sibling);
                    }
                }
            }
            Message::SplitPane(axis) => {
                if let Some(pane) = self.focus {
                    let result = self.panes.split(axis, &pane, Pane::new(Buffer::Empty));
                    if let Some((pane, _)) = result {
                        self.focus = Some(pane);
                    }
                }
            }
            Message::Pane(message) => {
                println!("pane message: {:?}", message);
            }
            Message::Buffer(_, _) => {
                println!("buffer message: {:?}", message);
            }
        }

        None
    }

    pub fn view<'a>(
        &'a self,
        clients: &data::client::Map,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let focus = self.focus;

        let pane_grid = PaneGrid::new(&self.panes, |id, pane| {
            let is_focused = focus == Some(id);
            let panes = self.panes.len();
            pane.view(
                theme,
                pane::Mapper {
                    pane: Message::Pane,
                    buffer: Message::Buffer,
                    on_close: Message::ClosePane,
                    on_split: |axis| Message::SplitPane(axis),
                },
                id,
                panes,
                is_focused,
                clients,
            )
        })
        .on_click(Message::PaneClicked)
        .on_resize(6, Message::PaneResized)
        .on_drag(Message::PaneDragged)
        .spacing(4);

        let pane_grid = container(pane_grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::container::primary(theme))
            .padding(8);

        column()
            .width(Length::Fill)
            .height(Length::Fill)
            .push(pane_grid)
            .into()
    }
}
