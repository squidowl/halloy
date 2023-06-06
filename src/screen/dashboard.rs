pub mod pane;
pub mod side_menu;

use pane::Pane;
use side_menu::SideMenu;

use data::config::Config;
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{container, row};
use iced::Length;
use iced::{keyboard, Command};

use crate::buffer::{self, channel, Buffer};
use crate::widget::Element;

pub struct Dashboard {
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    side_menu: SideMenu,
}

#[derive(Debug, Clone)]
pub enum Message {
    Pane(pane::Message),
    Buffer(pane_grid::Pane, buffer::Message),
    PaneDeselected,
    PaneClicked(pane_grid::Pane),
    PaneResized(pane_grid::ResizeEvent),
    PaneDragged(pane_grid::DragEvent),
    SideMenu(side_menu::Message),
    ClosePane,
    SplitPane(pane_grid::Axis),
    MaximizePane,
    Users,
}

pub enum Event {
    None,
}

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

        Dashboard {
            panes,
            focus: None,
            side_menu: SideMenu::new(),
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
    ) -> Option<(Event, Command<Message>)> {
        match message {
            Message::PaneClicked(pane) => {
                self.focus = Some(pane);

                None
            }
            Message::PaneDeselected => {
                self.focus = None;

                None
            }
            Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);

                None
            }
            Message::PaneDragged(pane_grid::DragEvent::Dropped {
                pane,
                target,
                region,
            }) => {
                self.panes.split_with(&target, &pane, region);

                None
            }
            Message::PaneDragged(_) => None,
            Message::ClosePane => {
                if let Some(pane) = self.focus {
                    if let Some((_, sibling)) = self.panes.close(&pane) {
                        self.focus = Some(sibling);
                    } else if let Some(pane) = self.panes.get_mut(&pane) {
                        pane.buffer = Buffer::Empty(Default::default());
                    }
                }

                None
            }
            Message::SplitPane(axis) => {
                if let Some(pane) = self.focus {
                    let result = self.panes.split(
                        axis,
                        &pane,
                        Pane::new(Buffer::Empty(buffer::empty::Empty::default())),
                    );
                    if let Some((pane, _)) = result {
                        self.focus = Some(pane);
                    }
                }

                None
            }
            Message::Pane(message) => {
                if let Some(pane) = self.focus {
                    if let Some(pane) = self.panes.get_mut(&pane) {
                        let command = pane.update(message);

                        return Some((Event::None, command.map(Message::Pane)));
                    }
                }

                None
            }
            Message::Buffer(pane, message) => {
                if let Some(pane) = self.panes.get_mut(&pane) {
                    let event = pane.buffer.update(message, clients)?;
                    match event {
                        buffer::Event::Empty(event) => match event {},
                        buffer::Event::Channel(event) => match event {},
                        buffer::Event::Server(event) => match event {},
                    }
                }

                None
            }
            Message::Users => {
                if let Some(pane) = self.focus {
                    if let Some(pane) = self.panes.get_mut(&pane) {
                        match &mut pane.buffer {
                            Buffer::Channel(state) => state.toggle_show_users(),
                            Buffer::Empty(_) => {}
                            Buffer::Server(_) => {}
                        }
                    }
                }

                None
            }
            Message::SideMenu(message) => {
                if let Some(event) = self.side_menu.update(message) {
                    let panes = self.panes.clone();

                    match event {
                        side_menu::Event::SelectChannel((server, channel)) => {
                            // If channel already is open, we focus it.
                            for (id, pane) in panes.iter() {
                                if let Buffer::Channel(state) = &pane.buffer {
                                    if state.server == server && state.channel == channel {
                                        self.focus = Some(*id);

                                        return None;
                                    }
                                }
                            }

                            // If we only have one pane, and its empty, we replace it.
                            if self.panes.len() == 1 {
                                for (id, pane) in panes.iter() {
                                    if let Buffer::Empty(_) = &pane.buffer {
                                        self.panes.panes.entry(*id).and_modify(|p| {
                                            *p = Pane::new(Buffer::Channel(channel::Channel::new(
                                                server.clone(),
                                                channel.clone(),
                                            )))
                                        });

                                        return None;
                                    }
                                }
                            }

                            // Default split could be a config option.
                            let axis = pane_grid::Axis::Horizontal;
                            let pane_to_split = {
                                if let Some(pane) = self.focus {
                                    pane
                                } else if let Some(pane) = self.panes.panes.keys().last() {
                                    *pane
                                } else {
                                    log::error!("Didn't find any panes");
                                    return None;
                                }
                            };

                            let result = self.panes.split(
                                axis,
                                &pane_to_split,
                                Pane::new(Buffer::Channel(channel::Channel::new(server, channel))),
                            );

                            if let Some((pane, _)) = result {
                                self.focus = Some(pane);
                            }
                        }
                        side_menu::Event::SelectServer(server) => {
                            // If server already is open, we focus it.
                            for (id, pane) in panes.iter() {
                                if let Buffer::Server(state) = &pane.buffer {
                                    if state.server == server {
                                        self.focus = Some(*id);

                                        return None;
                                    }
                                }
                            }

                            // If we only have one pane, and its empty, we replace it.
                            if self.panes.len() == 1 {
                                for (id, pane) in panes.iter() {
                                    if let Buffer::Empty(_) = &pane.buffer {
                                        self.panes.panes.entry(*id).and_modify(|p| {
                                            *p = Pane::new(Buffer::Server(
                                                buffer::server::Server::new(server.clone()),
                                            ))
                                        });

                                        return None;
                                    }
                                }
                            }

                            // Default split could be a config option.
                            let axis = pane_grid::Axis::Horizontal;
                            let pane_to_split = {
                                if let Some(pane) = self.focus {
                                    pane
                                } else if let Some(pane) = self.panes.panes.keys().last() {
                                    *pane
                                } else {
                                    log::error!("Didn't find any panes");
                                    return None;
                                }
                            };

                            let result = self.panes.split(
                                axis,
                                &pane_to_split,
                                Pane::new(Buffer::Server(buffer::server::Server::new(server))),
                            );

                            if let Some((pane, _)) = result {
                                self.focus = Some(pane);
                            }
                        }
                    }
                }

                None
            }
            Message::MaximizePane => {
                if self.panes.maximized().is_some() {
                    self.panes.restore();
                } else if let Some(pane) = self.focus {
                    self.panes.maximize(&pane);
                }

                None
            }
        }
    }

    pub fn view<'a>(&'a self, clients: &data::client::Map) -> Element<'a, Message> {
        let focus = self.focus;

        let pane_grid = PaneGrid::new(&self.panes, |id, pane, maximized| {
            let is_focused = focus == Some(id);
            let panes = self.panes.len();
            pane.view(
                pane::Mapper {
                    pane: Message::Pane,
                    buffer: Message::Buffer,
                    on_close: Message::ClosePane,
                    on_split: Message::SplitPane,
                    on_maximize: Message::MaximizePane,
                    on_users: Message::Users,
                },
                id,
                panes,
                is_focused,
                maximized,
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

        let side_menu = self.side_menu.view(clients).map(Message::SideMenu);

        row![side_menu, pane_grid]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn handle_keypress(
        &self,
        key_code: keyboard::KeyCode,
        _modifiers: keyboard::Modifiers,
    ) -> Option<Message> {
        match key_code {
            keyboard::KeyCode::Escape => {
                // Deselect pane if we have one selected.
                if self.focus.is_some() {
                    return Some(Message::PaneDeselected);
                }

                None
            }
            _ => None,
        }
    }
}
