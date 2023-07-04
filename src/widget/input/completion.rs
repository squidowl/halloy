use data::user::User;
use std::fmt;

use iced::widget::{column, container, row, text};

use crate::theme;
use crate::widget::Element;

#[derive(Debug, Clone, PartialEq)]
enum CompletionType {
    Command,
    User,
}

#[derive(Debug, Clone)]
pub struct Completion {
    selection: Selection,
    users: Vec<Entry>,
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
                    completion_type: CompletionType::Command,
                },
                Entry {
                    title: "MOTD",
                    args: vec![Arg {
                        text: "server",
                        optional: true,
                    }],
                    completion_type: CompletionType::Command,
                },
                Entry {
                    title: "NICK",
                    args: vec![Arg {
                        text: "nickname",
                        optional: false,
                    }],
                    completion_type: CompletionType::Command,
                },
                Entry {
                    title: "QUIT",
                    args: vec![Arg {
                        text: "reason",
                        optional: true,
                    }],
                    completion_type: CompletionType::Command,
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
                    completion_type: CompletionType::Command,
                },
                Entry {
                    title: "WHOIS",
                    args: vec![Arg {
                        text: "nick",
                        optional: false,
                    }],
                    completion_type: CompletionType::Command,
                },
                Entry {
                    title: "ME",
                    args: vec![Arg {
                        text: "action",
                        optional: false,
                    }],
                    completion_type: CompletionType::Command,
                },
                Entry {
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
                    completion_type: CompletionType::Command,
                },
                Entry {
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
                    completion_type: CompletionType::Command,
                },
                Entry {
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
                    completion_type: CompletionType::Command,
                },
                Entry {
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
                    completion_type: CompletionType::Command,
                },
                Entry {
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
                    completion_type: CompletionType::Command,
                },
            ],
            filtered_entries: vec![],
            users: vec![],
        }
    }
}

impl Completion {
    pub fn reset(&mut self) {
        self.filtered_entries = vec![];
        self.selection = Selection::None;
    }

    /// Convert a list of User structs into Completion Entries
    pub fn with_users(&mut self, users: Vec<User>) -> Self {
        self.users = users
            .iter()
            .map(|u| {
                // Let's make some eternal strings
                // TODO! Is this bad?
                let static_str = Box::leak(u.nickname().to_string().into_boxed_str());
                Entry {
                    title: static_str,
                    args: vec![],
                    completion_type: CompletionType::User,
                }
            })
            .collect();
        self.clone()
    }

    /// Determine if the current text is a viable candidate for completion
    /// If the command starts with a `/`, we can guess that the user is attempting
    /// to enter a /command.
    ///
    /// If the text doesn't have a `/`, then we want to rsplit the words so we can
    /// get the last word in the input, which can potentially be autocompleted
    fn valid_completion_input(input: &str, maybe_command: bool) -> Option<(&str, &str)> {
        if maybe_command {
            input.split_once('/')
        } else {
            // Are we looking at multiple words? Look at the last one
            match input.rsplit_once(' ') {
                Some((head, rest)) => Some((head, rest)),
                // Single command
                None => Some(("", input)),
            }
        }
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

    pub fn process(&mut self, input: &str, users: Vec<User>) {
        self.with_users(users);
        let maybe_command = input.starts_with('/');
        let Some((head, rest)) = Self::valid_completion_input(input, maybe_command) else {
            self.reset();
            return;
        };

        // Don't allow text before a command slash
        if maybe_command && !head.is_empty() {
            self.reset();
            return;
        }

        if !maybe_command && rest.is_empty() {
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
                    .chain(&self.users)
                    .filter(|entry| {
                        entry
                            .title
                            .to_lowercase()
                            .starts_with(&complete_candidate.to_lowercase())
                            && if maybe_command {
                                entry.completion_type == CompletionType::Command
                            } else {
                                entry.completion_type == CompletionType::User
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
                    .chain(&self.users)
                    .find(|entry| {
                        entry.title.to_lowercase() == complete_candidate.to_lowercase()
                            && if maybe_command {
                                entry.completion_type == CompletionType::Command
                            } else {
                                entry.completion_type == CompletionType::User
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
                    let command = match entry.completion_type {
                        CompletionType::Command => format!("/{}", entry.title),
                        CompletionType::User => entry.title.into(),
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
                            let content = text(match entry.completion_type {
                                CompletionType::Command => format!("/{}", entry.title),
                                CompletionType::User => entry.title.into(),
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
    completion_type: CompletionType,
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
