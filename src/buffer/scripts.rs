use std::collections::HashSet;

use data::scripts;
use iced::widget::{
    Scrollable, center, column, container, row, rule, scrollable, text, toggler,
};
use iced::{Alignment, Length, Task};

use crate::widget::Element;
use crate::{Theme, theme};

#[derive(Debug, Clone)]
pub enum Message {
    Toggle(String),
}

#[derive(Debug, Clone)]
pub enum Event {
    Toggle(String),
}

#[derive(Debug, Default, Clone)]
pub struct Scripts;

impl Scripts {
    pub fn new() -> Self {
        Self
    }

    pub fn update(
        &mut self,
        message: Message,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Toggle(name) => (Task::none(), Some(Event::Toggle(name))),
        }
    }
}

pub fn view<'a>(
    script_manager: &'a scripts::Manager,
    autorun: &[String],
    _theme: &'a Theme,
) -> Element<'a, Message> {
    let autorun: HashSet<_> = autorun.iter().cloned().collect();

    let mut entries: Vec<_> = script_manager
        .scripts()
        .map(|script| (script.name.clone(), script.is_loaded()))
        .collect();

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    if entries.is_empty() {
        return center(container(
            text("No scripts found").style(theme::text::secondary),
        ))
        .into();
    }

    let rows = column(entries.into_iter().enumerate().map(
        |(idx, (name, loaded))| {
            let is_autorun: bool = autorun.contains(&name);
            let name_clone = name.clone();
            let toggle = toggler(loaded)
                .on_toggle(move |_| Message::Toggle(name_clone.clone()));

            container(
                row![
                    row![
                        text(name).style(if loaded {
                            theme::text::primary
                        } else {
                            theme::text::secondary
                        }),
                        text(if is_autorun { "autorun" } else { "" })
                            .style(theme::text::tertiary)
                    ]
                    .spacing(6)
                    .width(Length::Fill),
                    toggle,
                ]
                .align_y(Alignment::Center)
                .spacing(10),
            )
            .padding([5, 8])
            .width(Length::Fill)
            .style(move |theme| theme::container::table(theme, idx + 1))
            .into()
        },
    ))
    .spacing(1)
    .padding([0, 2]);

    let content = column![
        rows,
        container(rule::horizontal(1))
            .padding([0, 2])
            .width(Length::Fill),
    ];

    container(
        Scrollable::new(content)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(1).scroller_width(1),
            ))
            .style(theme::scrollable::hidden),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
