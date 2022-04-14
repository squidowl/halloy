pub mod pane;
use pane::Pane;

use iced::pure::widget::pane_grid::{self, PaneGrid};
use iced::pure::Element;
use iced::pure::{column, container};
use iced::Command;
use iced::Length;

use crate::style;
use crate::theme::Theme;

pub struct Dashboard {
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Pane(pane::Message),
    PaneClicked(pane_grid::Pane),
    PaneResized(pane_grid::ResizeEvent),
    PaneDragged(pane_grid::DragEvent),
    ClosePane,
    SplitPane(pane_grid::Axis),
}

pub enum Event {}

impl Dashboard {
    pub fn new() -> Self {
        let (panes, _) = pane_grid::State::new(Pane::new(0));

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
                    let result = self
                        .panes
                        .split(axis, &pane, Pane::new(self.panes.len() + 1));
                    if let Some((pane, _)) = result {
                        self.focus = Some(pane);
                    }
                }
            }
            Message::Pane(message) => {
                println!("message: {:?}", message);
            }
        }

        None
    }

    pub fn view<'a>(&'a self, theme: &'a Theme) -> Element<'a, Message> {
        let focus = self.focus;

        let pane_grid = PaneGrid::new(&self.panes, |id, pane| {
            let is_focused = focus == Some(id);
            pane.view(
                theme,
                pane::Mapper {
                    pane: Message::Pane,
                    on_close: Message::ClosePane,
                    on_split: |axis| Message::SplitPane(axis),
                },
                id,
                is_focused,
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
