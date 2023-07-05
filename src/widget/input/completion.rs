use data::user::User;
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
                Entry::Command(CommandEntry {
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
                }),
                Entry::Command(CommandEntry {
                    title: "MOTD",
                    args: vec![Arg {
                        text: "server",
                        optional: true,
                    }],
                }),
                Entry::Command(CommandEntry {
                    title: "NICK",
                    args: vec![Arg {
                        text: "nickname",
                        optional: false,
                    }],
                }),
                Entry::Command(CommandEntry {
                    title: "QUIT",
                    args: vec![Arg {
                        text: "reason",
                        optional: true,
                    }],
                }),
                Entry::Command(CommandEntry {
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
                }),
                Entry::Command(CommandEntry {
                    title: "WHOIS",
                    args: vec![Arg {
                        text: "nick",
                        optional: false,
                    }],
                }),
                Entry::Command(CommandEntry {
                    title: "ME",
                    args: vec![Arg {
                        text: "action",
                        optional: false,
                    }],
                }),
                Entry::Command(CommandEntry {
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
                }),
                Entry::Command(CommandEntry {
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
                }),
                Entry::Command(CommandEntry {
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
                }),
                Entry::Command(CommandEntry {
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
                }),
                Entry::Command(CommandEntry {
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
                }),
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

    /// Convert a list of User structs into Completion Entries
    fn users_entries(users: &[User]) -> Vec<Entry> {
        users
            .iter()
            .map(|u| {
                // Let's make some eternal strings
                // TODO! Is this bad?
                Entry::User(UserEntry {
                    nickname: u.nickname().to_string(),
                })
            })
            .collect::<Vec<_>>()
    }

    /// If we are autocompleting a word, we want to replace the word we're completing with the
    /// completion target. This way, we can continue typing if we've, say, completed a users name
    ///
    /// # Examples
    ///
    /// ```
    /// let input = "Hello my good friend Ano";
    /// let completion = "AnonymousUser";
    /// let result = Completion::complete_selected_word(input, completion);
    /// assert!(result == "Hello my good friend AnonymousUser".to_string());
    /// ```
    pub fn complete_selected_word(input: &str, completion: &str) -> String {
        let Some((original_input, _completion_word)) = input.rsplit_once(' ') else {
            return completion.into()
        };
        format!("{} {}", original_input, completion)
    }

    /// If the entered text begins with a command char ('/'), then we want to look at the available
    /// command completions
    fn process_command(&mut self, input: &str) {
        let Some((head, rest)) = input.split_once('/') else {
            self.reset();
            return
        };

        // Don't allow text before a command slash
        if !head.is_empty() {
            self.reset();
            return;
        }

        let (complete_candidate, has_space) = if let Some(index) = rest.find(' ') {
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
                    .filter(|entry| {
                        if let Entry::Command(command) = entry {
                            command
                                .title
                                .to_lowercase()
                                .starts_with(&complete_candidate.to_lowercase())
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();
            }
            // Command fully typed, transition to showing known entry
            Selection::None | Selection::Highlighted(_) => {
                self.filtered_entries = vec![];
                if let Some(entry) = self
                    .entries
                    .iter()
                    .find(|entry| {
                        if let Entry::Command(command) = entry {
                            command.title.to_lowercase() == complete_candidate.to_lowercase()
                        } else {
                            false
                        }
                    })
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

    /// For any given word, we want to check if the user is attempting to autocomplete a name in a
    /// channel
    fn process_users(&mut self, input: &str, users: &[User]) {
        let user_entries = Self::users_entries(users);
        let (_, rest) = input.rsplit_once(' ').unwrap_or(("", input));

        // Empty input to operate on, ignore completions
        if rest.is_empty() {
            self.reset();
            return;
        }

        match self.selection {
            Selection::None => {
                self.selection = Selection::None;
                self.filtered_entries = user_entries
                    .iter()
                    .filter(|entry| {
                        if let Entry::User(user) = entry {
                            user.nickname.starts_with(&rest)
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();
            }
            // Command fully typed, transition to showing known entry
            Selection::Highlighted(_) => {
                self.filtered_entries = vec![];
                if let Some(entry) = user_entries
                    .iter()
                    .find(|entry| {
                        if let Entry::User(user) = entry {
                            user.nickname == rest
                        } else {
                            false
                        }
                    })
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

    /// Process input and
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
                    let command = match &entry {
                        Entry::Command(command) => format!("/{}", command.title),
                        Entry::User(user) => user.nickname.clone(),
                    };
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
                            let content = text(match &entry {
                                Entry::Command(command) => format!("/{}", command.title),
                                Entry::User(user) => user.nickname.clone(),
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
                Selection::Selected(entry) => Some(match entry {
                    Entry::Command(command) => command.view(input),
                    Entry::User(user) => user.view(),
                }),
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
pub enum Entry {
    Command(CommandEntry),
    User(UserEntry),
}

#[derive(Debug, Clone)]
pub struct CommandEntry {
    title: &'static str,
    args: Vec<Arg>,
}

#[derive(Debug, Clone)]
pub struct UserEntry {
    nickname: String,
}

/// Dictates how we render a user completion.
impl UserEntry {
    pub fn view<'a, Message: 'a>(&self) -> Element<'a, Message> {
        let nick = Some(Element::from(text(self.nickname.clone())));

        container(row(nick.into_iter().collect()))
            .style(theme::Container::Context)
            .padding(8)
            .center_y()
            .into()
    }
}

/// Dictates how we render a command completion. A command may or may not have args,
/// so we want to include those as autocomplete hints when the completion is selected
impl CommandEntry {
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
