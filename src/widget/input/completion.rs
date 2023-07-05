use data::user::User;
use std::fmt;

use iced::widget::{column, container, row, text};

use crate::theme;
use crate::widget::Element;

const MAX_SHOWN_ENTRIES: usize = 5;

#[derive(Debug, Clone)]
pub struct Completion {
    selection: Selection,
    commands: Vec<Command>,
    filtered_entries: Vec<Entry>,
}

impl Default for Completion {
    fn default() -> Self {
        Self {
            selection: Selection::None,
            // TODO: Macro magic all commands as entries or manually add them all :(
            commands: vec![
                Command {
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
                Command {
                    title: "MOTD",
                    args: vec![Arg {
                        text: "server",
                        optional: true,
                    }],
                },
                Command {
                    title: "NICK",
                    args: vec![Arg {
                        text: "nickname",
                        optional: false,
                    }],
                },
                Command {
                    title: "QUIT",
                    args: vec![Arg {
                        text: "reason",
                        optional: true,
                    }],
                },
                Command {
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
                Command {
                    title: "WHOIS",
                    args: vec![Arg {
                        text: "nick",
                        optional: false,
                    }],
                },
                Command {
                    title: "ME",
                    args: vec![Arg {
                        text: "action",
                        optional: false,
                    }],
                },
                Command {
                    title: "MODE",
                    args: vec![
                        Arg {
                            text: "channel",
                            optional: false,
                        },
                        Arg {
                            text: "mode",
                            optional: false,
                        },
                        Arg {
                            text: "user",
                            optional: true,
                        },
                    ],
                },
                Command {
                    title: "PART",
                    args: vec![
                        Arg {
                            text: "channels",
                            optional: false,
                        },
                        Arg {
                            text: "reason",
                            optional: true,
                        },
                    ],
                },
                Command {
                    title: "TOPIC",
                    args: vec![
                        Arg {
                            text: "channel",
                            optional: false,
                        },
                        Arg {
                            text: "topic",
                            optional: true,
                        },
                    ],
                },
                Command {
                    title: "KICK",
                    args: vec![
                        Arg {
                            text: "channel",
                            optional: false,
                        },
                        Arg {
                            text: "user",
                            optional: false,
                        },
                        Arg {
                            text: "comment",
                            optional: true,
                        },
                    ],
                },
                Command {
                    title: "RAW",
                    args: vec![
                        Arg {
                            text: "command",
                            optional: false,
                        },
                        Arg {
                            text: "args",
                            optional: true,
                        },
                    ],
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

    /// If the entered text begins with a command char ('/') then we want to populate
    /// applicable command completions
    fn process_command(&mut self, input: &str) {
        let Some((head, rest)) = input.split_once('/') else {
            self.reset();
            return;
        };

        // Don't allow text before a command slash
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
                    .commands
                    .iter()
                    .filter(|command| {
                        command
                            .title
                            .to_lowercase()
                            .starts_with(&cmd.to_lowercase())
                    })
                    .cloned()
                    .map(Entry::Command)
                    .collect();
            }
            // Command fully typed, transition to showing known entry
            Selection::None | Selection::Highlighted(_) => {
                self.filtered_entries = vec![];
                if let Some(command) = self
                    .commands
                    .iter()
                    .find(|command| command.title.to_lowercase() == cmd.to_lowercase())
                    .cloned()
                {
                    self.selection = Selection::SelectedCommand(command);
                } else {
                    self.selection = Selection::None;
                }
            }
            // Command fully typed & already selected, do nothing
            Selection::SelectedCommand(_) => {}
        }
    }

    /// If the trailing word starts with an @ we want to populate applicable user completions
    fn process_users(&mut self, input: &str, users: &[User]) {
        let (_, rest) = input.rsplit_once(' ').unwrap_or(("", input));

        // Only show user completions if using @
        let Some((_, nick)) = rest.split_once('@') else {
            self.reset();
            return;
        };

        match self.selection {
            Selection::None | Selection::Highlighted(_) => {
                self.selection = Selection::None;
                self.filtered_entries = users
                    .iter()
                    .filter_map(|user| {
                        let nickname = user.nickname();
                        nickname
                            .as_ref()
                            .starts_with(nick)
                            .then(|| nickname.to_string())
                    })
                    .map(Entry::User)
                    .collect();
            }
            Selection::SelectedCommand(_) => {}
        }
    }

    /// Process input and update the completion state
    pub fn process(&mut self, input: &str, users: &[User]) {
        if input.starts_with('/') {
            self.process_command(input);
        } else {
            self.process_users(input, users);
        }
    }

    pub fn is_selecting(&self) -> bool {
        match self.selection {
            Selection::None | Selection::Highlighted(_) => !self.filtered_entries.is_empty(),
            Selection::SelectedCommand(_) => false,
        }
    }

    fn is_active(&self) -> bool {
        match self.selection {
            Selection::None | Selection::Highlighted(_) => !self.filtered_entries.is_empty(),
            Selection::SelectedCommand(_) => true,
        }
    }

    pub fn select(&mut self) -> Option<Entry> {
        match self.selection {
            Selection::None => {
                self.filtered_entries = vec![];
            }
            Selection::Highlighted(index) => {
                if let Some(entry) = self.filtered_entries.get(index).cloned() {
                    self.filtered_entries = vec![];

                    if let Entry::Command(command) = &entry {
                        self.selection = Selection::SelectedCommand(command.clone());
                    }

                    return Some(entry);
                }
            }
            Selection::SelectedCommand(_) => {}
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
                    let skip = {
                        let index = if let Selection::Highlighted(index) = &self.selection {
                            *index
                        } else {
                            0
                        };

                        let to = index.max(MAX_SHOWN_ENTRIES - 1);
                        to.saturating_sub(MAX_SHOWN_ENTRIES - 1)
                    };

                    let entries = self
                        .filtered_entries
                        .iter()
                        .enumerate()
                        .skip(skip)
                        .take(MAX_SHOWN_ENTRIES)
                        .map(|(index, entry)| {
                            let selected = Some(index) == self.selection.highlighted();
                            let content = text(match &entry {
                                Entry::Command(command) => format!("/{}", command.title),
                                Entry::User(nickname) => nickname.clone(),
                            });

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
                Selection::SelectedCommand(command) => Some(command.view(input)),
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
    SelectedCommand(Command),
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
pub enum Entry {
    Command(Command),
    User(String),
}

impl Entry {
    pub fn complete_input(&self, input: &str) -> String {
        match self {
            Entry::Command(command) => format!("/{}", command.title),
            Entry::User(nickname) => match input.rsplit_once(' ') {
                Some((left, _)) => format!("{left} {nickname}"),
                None => nickname.clone(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    title: &'static str,
    args: Vec<Arg>,
}

impl Command {
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
