pub mod pane;
pub mod side_menu;

use data::{history, message, Server};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{container, row};
use iced::{clipboard, subscription, window, Command, Length, Subscription};
use pane::Pane;
use side_menu::SideMenu;

use crate::buffer::{self, Buffer};
use crate::widget::{selectable_text, Element};

pub struct Dashboard {
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    side_menu: SideMenu,
    history: history::Manager,
}

#[derive(Debug)]
pub enum Message {
    Pane(pane::Message),
    SideMenu(side_menu::Message),
    SelectedText(Vec<(f32, String)>),
    History(history::manager::Message),
    Close,
}

pub enum Event {
    SaveSettings,
}

impl Dashboard {
    pub fn new() -> (Self, Command<Message>) {
        let buffers = vec![];

        let first_buffer = Buffer::Empty(Default::default());

        let (mut panes, pane) = pane_grid::State::new(Pane::new(first_buffer));

        for buffer in buffers.into_iter().rev() {
            panes.split(pane_grid::Axis::Horizontal, &pane, Pane::new(buffer));
        }

        let mut dashboard = Dashboard {
            panes,
            focus: None,
            side_menu: SideMenu::new(),
            history: history::Manager::default(),
        };

        let command = dashboard.track();

        (dashboard, command)
    }

    pub fn update(
        &mut self,
        message: Message,
        clients: &mut data::client::Map,
        config: &mut data::config::Config,
    ) -> (Command<Message>, Option<Event>) {
        match message {
            Message::Pane(message) => match message {
                pane::Message::PaneClicked(pane) => {
                    return (self.focus_pane(pane), None);
                }
                pane::Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                    self.panes.resize(&split, ratio);
                }
                pane::Message::PaneDragged(pane_grid::DragEvent::Dropped {
                    pane,
                    target,
                    region,
                }) => {
                    self.panes.split_with(&target, &pane, region);
                }
                pane::Message::PaneDragged(_) => {}
                pane::Message::ClosePane => {
                    if let Some(pane) = self.focus {
                        if let Some((_, sibling)) = self.panes.close(&pane) {
                            return (self.focus_pane(sibling), None);
                        } else if let Some(pane) = self.panes.get_mut(&pane) {
                            pane.buffer = Buffer::Empty(Default::default());
                        }
                    }
                }
                pane::Message::SplitPane(axis) => {
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
                pane::Message::Buffer(id, message) => {
                    if let Some(pane) = self.panes.get_mut(&id) {
                        let (command, event) =
                            pane.buffer.update(message, clients, &mut self.history);

                        match event {
                            Some(buffer::Event::Empty(event)) => match event {},
                            Some(buffer::Event::Channel(event)) => match event {},
                            Some(buffer::Event::Server(event)) => match event {},
                            Some(buffer::Event::Query(event)) => match event {},
                            None => {}
                        }

                        return (
                            command.map(move |message| {
                                Message::Pane(pane::Message::Buffer(id, message))
                            }),
                            None,
                        );
                    }
                }
                pane::Message::ToggleShowUserList => {
                    if let Some((_, pane)) = self.get_focused_mut() {
                        match &mut pane.buffer {
                            Buffer::Channel(state) => {
                                let config =
                                    config.channel_config_mut(&state.server.name, &state.channel);

                                config.users.toggle_visibility();
                                return (Command::none(), Some(Event::SaveSettings));
                            }
                            Buffer::Empty(_) => {}
                            Buffer::Server(_) => {}
                            Buffer::Query(_) => {}
                        }
                    }
                }
                pane::Message::MaximizePane => {
                    if self.panes.maximized().is_some() {
                        self.panes.restore();
                    } else if let Some(pane) = self.focus {
                        self.panes.maximize(&pane);
                    }
                }
            },
            Message::SideMenu(message) => {
                if let Some(event) = self.side_menu.update(message) {
                    let panes = self.panes.clone();

                    match event {
                        side_menu::Event::Open(kind) => {
                            // If channel already is open, we focus it.
                            for (id, pane) in panes.iter() {
                                if pane.buffer.kind().as_ref() == Some(&kind) {
                                    self.focus = Some(*id);

                                    return (self.focus_pane(*id), None);
                                }
                            }

                            // If we only have one pane, and its empty, we replace it.
                            if self.panes.len() == 1 {
                                for (id, pane) in panes.iter() {
                                    if let Buffer::Empty(_) = &pane.buffer {
                                        self.panes
                                            .panes
                                            .entry(*id)
                                            .and_modify(|p| *p = Pane::new(Buffer::from(kind)));

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
                                Pane::new(Buffer::from(kind)),
                            );

                            if let Some((pane, _)) = result {
                                return (self.focus_pane(pane), None);
                            }
                        }
                        side_menu::Event::Replace(kind, pane) => {
                            if let Some(state) = self.panes.get_mut(&pane) {
                                state.buffer = Buffer::from(kind);
                                return (self.focus_pane(pane), None);
                            }
                        }
                        side_menu::Event::Close(pane) => {
                            self.panes.close(&pane);

                            if self.focus == Some(pane) {
                                self.focus = None;
                            }
                        }
                        side_menu::Event::Swap(from, to) => {
                            self.panes.swap(&from, &to);
                            return (self.focus_pane(from), None);
                        }
                    }
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
            Message::History(message) => {
                let command = Command::batch(
                    self.history
                        .update(message)
                        .into_iter()
                        .map(|task| Command::perform(task, Message::History))
                        .collect::<Vec<_>>(),
                );

                return (command, None);
            }
            Message::Close => {
                return (window::close(), None);
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

        let pane_grid: Element<_> = PaneGrid::new(&self.panes, |id, pane, maximized| {
            let is_focused = focus == Some(id);
            let panes = self.panes.len();
            pane.view(
                id,
                panes,
                is_focused,
                maximized,
                clients,
                &self.history,
                config,
            )
        })
        .on_click(pane::Message::PaneClicked)
        .on_resize(6, pane::Message::PaneResized)
        .on_drag(pane::Message::PaneDragged)
        .spacing(4)
        .into();

        let pane_grid = container(pane_grid.map(Message::Pane))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8);

        let side_menu = self
            .side_menu
            .view(clients, &self.history, &self.panes, self.focus)
            .map(Message::SideMenu);

        // The height margin varies across different operating systems due to design differences.
        // For instance, on macOS, the menubar is hidden, resulting in a need for additional padding to accommodate the
        // space occupied by the traffic light buttons.
        let height_margin = if cfg!(target_os = "macos") { 20 } else { 0 };

        row![side_menu, pane_grid]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([height_margin, 0, 0, 0])
            .into()
    }

    pub fn handle_event(&mut self, event: crate::event::Event) -> Command<Message> {
        use crate::event::Event::*;

        match event {
            Escape => {
                self.focus = None;
                Command::none()
            }
            Copy => selectable_text::selected(Message::SelectedText),
            Home => self
                .get_focused_mut()
                .map(|(id, pane)| {
                    pane.buffer
                        .scroll_to_start()
                        .map(move |message| Message::Pane(pane::Message::Buffer(id, message)))
                })
                .unwrap_or_else(Command::none),
            End => self
                .get_focused_mut()
                .map(|(pane, state)| {
                    state
                        .buffer
                        .scroll_to_end()
                        .map(move |message| Message::Pane(pane::Message::Buffer(pane, message)))
                })
                .unwrap_or_else(Command::none),
            CloseRequested => Command::perform(self.history.close(), |_| Message::Close),
        }
    }

    pub fn messages_received(
        &mut self,
        messages: Vec<(Server, message::Raw)>,
        clients: &mut data::client::Map,
    ) -> Command<Message> {
        let _ = self.history.add_raw_messages(messages, clients);
        Command::none()
    }

    fn get_focused_mut(&mut self) -> Option<(pane_grid::Pane, &mut Pane)> {
        let pane = self.focus?;
        self.panes.get_mut(&pane).map(|state| (pane, state))
    }

    fn focus_pane(&mut self, pane: pane_grid::Pane) -> Command<Message> {
        if self.focus != Some(pane) {
            self.focus = Some(pane);

            self.panes
                .iter()
                .find_map(|(p, state)| {
                    (*p == pane).then(|| {
                        state
                            .buffer
                            .focus()
                            .map(move |message| Message::Pane(pane::Message::Buffer(pane, message)))
                    })
                })
                .unwrap_or(Command::none())
        } else {
            Command::none()
        }
    }

    pub fn track(&mut self) -> Command<Message> {
        let resources = self
            .panes
            .iter()
            .filter_map(|(_, pane)| pane.resource())
            .collect();

        Command::batch(
            self.history
                .track(resources)
                .into_iter()
                .map(|fut| Command::perform(fut, Message::History))
                .collect::<Vec<_>>(),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        subscription::run(history::manager::tick).map(Message::History)
    }
}
