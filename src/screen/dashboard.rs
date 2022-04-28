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
        let mut buffers: Vec<Buffer> = Vec::new();
        for server in config.servers.iter() {
            buffers.push(Buffer::Server(buffer::server::State::new(
                server.server.clone().unwrap().into(),
            )));

            for channel in server.channels() {
                buffers.push(Buffer::Channel(buffer::channel::State::new(
                    server.server.clone().unwrap().into(),
                    channel.as_str().parse().unwrap(),
                )));
            }
        }

        let first_buffer = if buffers.len() > 0 {
            buffers.remove(0)
        } else {
            Buffer::Empty
        };

        let (mut panes, pane) = pane_grid::State::new(Pane::new(first_buffer));

        for buffer in buffers {
            panes.split(pane_grid::Axis::Horizontal, &pane, Pane::new(buffer));
        }

        Dashboard { panes, focus: None }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &data::client::Map,
    ) -> Option<(Event, Command<Message>)> {
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
            Message::Buffer(pane, message) => {
                if let Some(pane) = self.panes.get_mut(&pane) {
                    pane.buffer.update(message, clients);
                }
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
