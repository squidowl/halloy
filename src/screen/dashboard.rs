pub mod pane;
pub mod side_menu;

use data::config::Config;
use data::message;
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{container, row};
use iced::{clipboard, keyboard, Command, Length};
use pane::Pane;
use side_menu::SideMenu;

use crate::buffer::{self, channel, Buffer};
use crate::widget::{selectable_text, Element};

pub struct Dashboard {
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    side_menu: SideMenu,
}

#[derive(Debug, Clone)]
pub enum Message {
    Pane(pane::Message),
    Buffer(pane_grid::Pane, buffer::Message),
    PaneClicked(pane_grid::Pane),
    PaneResized(pane_grid::ResizeEvent),
    PaneDragged(pane_grid::DragEvent),
    SideMenu(side_menu::Message),
    ClosePane,
    SplitPane(pane_grid::Axis),
    MaximizePane,
    ToggleShowUserList,
    SelectedText(Vec<(f32, String)>),
}

pub enum Event {
    SaveSettings,
}

impl Dashboard {
    pub fn new(_config: &Config) -> Self {
        let buffers = vec![];

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
        config: &mut data::config::Config,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::PaneClicked(pane) => {
                return (self.focus_pane(pane), None);
            }
            Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);
            }
            Message::PaneDragged(pane_grid::DragEvent::Dropped {
                pane,
                target,
                region,
            }) => {
                self.panes.split_with(&target, &pane, region);
            }
            Message::PaneDragged(_) => {}
            Message::ClosePane => {
                if let Some(pane) = self.focus {
                    if let Some((_, sibling)) = self.panes.close(&pane) {
                        return (self.focus_pane(sibling), None);
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
                        Pane::new(Buffer::Empty(buffer::empty::Empty::default())),
                    );
                    if let Some((pane, _)) = result {
                        return (self.focus_pane(pane), None);
                    }
                }
            }
            Message::Pane(message) => {
                if let Some(pane) = self.get_focused_mut() {
                    let command = pane.update(message);

                    return (command.map(Message::Pane), None);
                }
            }
            Message::Buffer(id, message) => {
                if let Some(pane) = self.panes.get_mut(&id) {
                    let (command, event) = pane.buffer.update(message, clients);

                    match event {
                        Some(buffer::Event::Empty(event)) => match event {},
                        Some(buffer::Event::Channel(event)) => match event {},
                        Some(buffer::Event::Server(event)) => match event {},
                        None => {}
                    }

                    return (
                        command.map(move |message| Message::Buffer(id, message)),
                        None,
                    );
                }
            }
            Message::ToggleShowUserList => {
                if let Some(pane) = self.get_focused_mut() {
                    match &mut pane.buffer {
                        Buffer::Channel(state) => {
                            let config =
                                config.channel_config_mut(&state.server.name, &state.channel);

                            config.users.toggle_visibility();
                            return (Command::none(), Some(Event::SaveSettings));
                        }
                        Buffer::Empty(_) => {}
                        Buffer::Server(_) => {}
                    }
                }
            }
            Message::SideMenu(message) => {
                if let Some(event) = self.side_menu.update(message) {
                    let panes = self.panes.clone();

                    // TODO: Repetitive code below. Should be combined into one.
                    match event {
                        side_menu::Event::SelectChannel((server, channel)) => {
                            // If channel already is open, we focus it.
                            for (id, pane) in panes.iter() {
                                if let Buffer::Channel(state) = &pane.buffer {
                                    if state.server == server && state.channel == channel {
                                        self.focus = Some(*id);

                                        return (self.focus_pane(*id), None);
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

                                        return (self.focus_pane(*id), None);
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
                                    return (Command::none(), None);
                                }
                            };

                            let result = self.panes.split(
                                axis,
                                &pane_to_split,
                                Pane::new(Buffer::Channel(channel::Channel::new(server, channel))),
                            );

                            if let Some((pane, _)) = result {
                                return (self.focus_pane(pane), None);
                            }
                        }
                        side_menu::Event::SelectServer(server) => {
                            // If server already is open, we focus it.
                            for (id, pane) in panes.iter() {
                                if let Buffer::Server(state) = &pane.buffer {
                                    if state.server == server {
                                        return (self.focus_pane(*id), None);
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

                                        return (self.focus_pane(*id), None);
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
                                    return (Command::none(), None);
                                }
                            };

                            let result = self.panes.split(
                                axis,
                                &pane_to_split,
                                Pane::new(Buffer::Server(buffer::server::Server::new(server))),
                            );

                            if let Some((pane, _)) = result {
                                return (self.focus_pane(pane), None);
                            }
                        }
                    }
                }
            }
            Message::MaximizePane => {
                if self.panes.maximized().is_some() {
                    self.panes.restore();
                } else if let Some(pane) = self.focus {
                    self.panes.maximize(&pane);
                }
            }
            Message::SelectedText(contents) => {
                let mut last_y = None;
                let contents = contents
                    .into_iter()
                    .fold(String::new(), |acc, (y, content)| {
                        if let Some(_y) = last_y {
                            let new_line = if y == _y { "" } else { "\n" };
                            last_y = Some(y);

                            format!("{acc}{new_line}{content}")
                        } else {
                            last_y = Some(y);

                            content
                        }
                    });

                return (clipboard::write(contents), None);
            }
        }

        (Command::none(), None)
    }

    pub fn view<'a>(
        &'a self,
        clients: &'a data::client::Map,
        config: &'a data::config::Config,
    ) -> Element<'a, Message> {
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
                    on_users: Message::ToggleShowUserList,
                },
                id,
                panes,
                is_focused,
                maximized,
                clients,
                config,
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

        let side_menu = self
            .side_menu
            .view(clients, &self.panes)
            .map(Message::SideMenu);

        row![side_menu, pane_grid]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn handle_keypress(
        &mut self,
        key_code: keyboard::KeyCode,
        modifiers: keyboard::Modifiers,
    ) -> Command<Message> {
        match key_code {
            keyboard::KeyCode::Escape => {
                // Deselect pane if we have one selected.
                if self.focus.is_some() {
                    self.focus = None;
                }

                Command::none()
            }
            keyboard::KeyCode::C if modifiers.command() => {
                selectable_text::selected(Message::SelectedText)
            }
            _ => Command::none(),
        }
    }

    pub fn message_received(
        &self,
        _server: &data::Server,
        _source: message::Source,
    ) -> Command<Message> {
        // TODO: Placeholder for message related hooks
        Command::none()
    }

    fn get_focused_mut(&mut self) -> Option<&mut Pane> {
        let pane = self.focus?;
        self.panes.get_mut(&pane)
    }

    fn focus_pane(&mut self, pane: pane_grid::Pane) -> Command<Message> {
        self.focus = Some(pane);

        self.panes
            .iter()
            .find_map(|(p, state)| {
                (*p == pane).then(|| {
                    state
                        .buffer
                        .focus()
                        .map(move |message| Message::Buffer(pane, message))
                })
            })
            .unwrap_or(Command::none())
    }
}
