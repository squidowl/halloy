pub mod pane;

use data::server::Server;
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
    Users(Server, String),
}

pub enum Event {}

impl Dashboard {
    pub fn new(config: &Config) -> Self {
        let mut buffers = vec![];

        for server_config in config.servers.iter() {
            buffers.push(Buffer::Server(buffer::server::State::new(
                server_config.server.clone().unwrap_or_default().into(),
            )));

            for channel in server_config.channels() {
                buffers.push(Buffer::Channel(buffer::channel::State::new(
                    server_config.server.clone().unwrap_or_default().into(),
                    channel.clone(),
                )));
            }
        }

        buffers.push(Buffer::Empty(buffer::empty::State::default()));

        let first_buffer = if !buffers.is_empty() {
            buffers.remove(0)
        } else {
            Buffer::Empty(buffer::empty::State::default())
        };

        let (mut panes, pane) = pane_grid::State::new(Pane::new(first_buffer));

        for buffer in buffers.into_iter().rev() {
            panes.split(pane_grid::Axis::Horizontal, &pane, Pane::new(buffer));
        }

        Dashboard { panes, focus: None }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
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
                    let result = self.panes.split(
                        axis,
                        &pane,
                        Pane::new(Buffer::Empty(buffer::empty::State::default())),
                    );
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
            Message::Users(server, channel) => {
                if let Some(pane) = self.focus {
                    let result = self.panes.split(
                        iced::pane_grid::Axis::Vertical,
                        &pane,
                        Pane::new(Buffer::Users(buffer::users::State::new(server, channel))),
                    );
                    if let Some((pane, _)) = result {
                        self.focus = Some(pane);
                    }
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
                    on_split: Message::SplitPane,
                    on_users: Message::Users,
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
