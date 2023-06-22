use std::fmt;

use iced::widget::{column, container, row, text};

use crate::theme;
use crate::widget::Element;

#[derive(Debug, Clone)]
pub struct Completion {
    selection: Selection,
    entries: Vec<Entry>,
    filtered_entries: Vec<Entry>,
}

impl Default for Completion {
    fn default() -> Self {
        Self {
            selection: Selection::None,
            // TODO: Macro magic all commands as entries or manually add them all :(
            entries: vec![
                Entry {
                    title: "JOIN",
                    args: vec![
                        Arg {
                            text: "channels",
                            optional: false,
                        },
                        Arg {
                            text: "keys",
                            optional: true,
                        },
                    ],
                },
                Entry {
                    title: "MOTD",
                    args: vec![Arg {
                        text: "server",
                        optional: true,
                    }],
                },
                Entry {
                    title: "NICK",
                    args: vec![Arg {
                        text: "nickname",
                        optional: false,
                    }],
                },
                Entry {
                    title: "QUIT",
                    args: vec![Arg {
                        text: "reason",
                        optional: true,
                    }],
                },
                Entry {
                    title: "MSG",
                    args: vec![
                        Arg {
                            text: "target",
                            optional: false,
                        },
                        Arg {
                            text: "text",
                            optional: false,
                        },
                    ],
                },
                Entry {
                    title: "ME",
                    args: vec![Arg {
                        text: "action",
                        optional: false,
                    }],
                },
            ],
            filtered_entries: vec![],
        }
    }
}

impl Completion {
    pub fn reset(&mut self) {
        self.filtered_entries = vec![];
        self.selection = Selection::None;
    }

    pub fn process(&mut self, input: &str) {
        let Some((head, rest)) = input.split_once('/') else {
            self.reset();
            return;
        };
        // Don't allow leading whitespace before slash
        if !head.is_empty() {
            self.reset();
            return;
        }

        let (cmd, has_space) = if let Some(index) = rest.find(' ') {
            (&rest[0..index], true)
        } else {
            (rest, false)
        };

        match self.selection {
            // Command not fully typed, show filtered entries
            _ if !has_space => {
                self.selection = Selection::None;
                self.filtered_entries = self
                    .entries
                    .iter()
                    .filter(|entry| entry.title.to_lowercase().starts_with(&cmd.to_lowercase()))
                    .cloned()
                    .collect();
            }
            // Command fully typed, transition to showing known entry
            Selection::None | Selection::Highlighted(_) => {
                self.filtered_entries = vec![];
                if let Some(entry) = self
                    .entries
                    .iter()
                    .find(|entry| entry.title.to_lowercase() == cmd.to_lowercase())
                    .cloned()
                {
                    self.selection = Selection::Selected(entry);
                } else {
                    self.selection = Selection::None;
                }
            }
            // Command fully typed & already selected, do nothing
            Selection::Selected(_) => {}
        }
    }

    pub fn is_selecting(&self) -> bool {
        match self.selection {
            Selection::None | Selection::Highlighted(_) => !self.filtered_entries.is_empty(),
            Selection::Selected(_) => false,
        }
    }

    fn is_active(&self) -> bool {
        match self.selection {
            Selection::None | Selection::Highlighted(_) => !self.filtered_entries.is_empty(),
            Selection::Selected(_) => true,
        }
    }

    pub fn select(&mut self) -> Option<String> {
        match self.selection {
            Selection::None => {
                self.filtered_entries = vec![];
            }
            Selection::Highlighted(index) => {
                if let Some(entry) = self.filtered_entries.get(index).cloned() {
                    let command = format!("/{}", entry.title);
                    self.filtered_entries = vec![];
                    self.selection = Selection::Selected(entry);
                    return Some(command);
                }
            }
            Selection::Selected(_) => {}
        }
        None
    }

    pub fn tab(&mut self) {
        if let Selection::Highlighted(index) = &mut self.selection {
            *index = (*index + 1) % self.filtered_entries.len();
        } else if matches!(self.selection, Selection::None) {
            self.selection = Selection::Highlighted(0);
        }
    }

    pub fn view<'a, Message: 'a>(&self, input: &str) -> Option<Element<'a, Message>> {
        if self.is_active() {
            match &self.selection {
                Selection::None | Selection::Highlighted(_) => {
                    let entries = self
                        .filtered_entries
                        .iter()
                        .enumerate()
                        .map(|(index, entry)| {
                            let selected = Some(index) == self.selection.highlighted();
                            let content = text(format!("/{}", entry.title));

                            Element::from(
                                container(content)
                                    .style(theme::Container::Command { selected })
                                    .padding(6)
                                    .center_y(),
                            )
                        })
                        .collect();

                    Some(
                        container(column(entries))
                            .padding(4)
                            .style(theme::Container::Context)
                            .into(),
                    )
                }
                Selection::Selected(entry) => Some(entry.view(input)),
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
enum Selection {
    None,
    Highlighted(usize),
    Selected(Entry),
}

impl Selection {
    fn highlighted(&self) -> Option<usize> {
        if let Self::Highlighted(index) = self {
            Some(*index)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    title: &'static str,
    args: Vec<Arg>,
}

impl Entry {
    pub fn view<'a, Message: 'a>(&self, input: &str) -> Element<'a, Message> {
        let active_arg = [input, "_"]
            .concat()
            .split_ascii_whitespace()
            .count()
            .saturating_sub(2)
            .min(self.args.len().saturating_sub(1));

        let title = Some(Element::from(text(self.title)));

        let args = self.args.iter().enumerate().map(|(index, arg)| {
            let style = if index == active_arg {
                theme::Text::Accent
            } else {
                theme::Text::Default
            };

            Element::from(text(format!(" {arg}")).style(style))
        });

        container(row(title.into_iter().chain(args).collect()))
            .style(theme::Container::Context)
            .padding(8)
            .center_y()
            .into()
    }
}

#[derive(Debug, Clone)]
pub struct Arg {
    text: &'static str,
    optional: bool,
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.optional {
            write!(f, "[<{}>]", self.text)
        } else {
            write!(f, "<{}>", self.text)
        }
    }
}
