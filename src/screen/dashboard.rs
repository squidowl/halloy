pub mod pane;

use data::server::Server;
use pane::Pane;

use data::config::Config;
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{column, container};
use iced::Command;
use iced::Length;

use crate::buffer::{self, Buffer};
use crate::theme;
use crate::widget::Element;

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
    Users,
}

pub enum Event {}

impl Dashboard {
    pub fn new(config: &Config) -> Self {
        let mut buffers = vec![];

        // for server_config in config.servers.iter() {
        //     buffers.push(Buffer::Server(buffer::server::State::new(
        //         server_config.server.clone().unwrap_or_default().into(),
        //     )));

        //     for channel in server_config.channels() {
        //         buffers.push(Buffer::Channel(buffer::channel::State::new(
        //             server_config.server.clone().unwrap_or_default().into(),
        //             channel.clone(),
        //         )));
        //     }
        // }

        // buffers.push(Buffer::Empty(Default::default()));

        // let first_buffer = if !buffers.is_empty() {
        //     buffers.remove(0)
        // } else {
        //     Buffer::Empty(Default::default())
        // };

        let first_buffer = Buffer::Empty(Default::default());

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
            Message::PaneDragged(pane_grid::DragEvent::Dropped {
                pane,
                target,
                region,
            }) => {
                self.panes.swap(&pane, &target);
            }
            Message::PaneDragged(_) => {}
            Message::ClosePane => {
                if let Some(pane) = self.focus {
                    if let Some((_, sibling)) = self.panes.close(&pane) {
                        self.focus = Some(sibling);
                    } else if let Some(pane) = self.panes.get_mut(&pane) {
                        pane.buffer = Buffer::Empty(Default::default());
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
                    let event = pane.buffer.update(message, clients)?;
                    match event {
                        buffer::Event::Empty(event) => match event {
                            buffer::empty::Event::SelectChannel((server, channel)) => {
                                pane.buffer =
                                    Buffer::Channel(buffer::channel::State::new(server, channel));
                            }
                            buffer::empty::Event::SelectServer(server) => {
                                pane.buffer = Buffer::Server(buffer::server::State::new(server));
                            }
                        },
                        buffer::Event::Channel(_event) => {}
                    }
                }
            }
            Message::Users => {
                if let Some(pane) = self.focus {
                    if let Some(pane) = self.panes.get_mut(&pane) {
                        match &mut pane.buffer {
                            Buffer::Channel(state) => state.toggle_show_users(),
                            _ => (),
                        }
                    }
                }
            }
        }

        None
    }

    pub fn view<'a>(&'a self, clients: &data::client::Map) -> Element<'a, Message> {
        let focus = self.focus;

        let pane_grid = PaneGrid::new(&self.panes, |id, pane, _maximized| {
            let is_focused = focus == Some(id);
            let panes = self.panes.len();
            pane.view(
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
            .padding(8);

        column![pane_grid]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
