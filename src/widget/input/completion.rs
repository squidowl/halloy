use std::fmt;

use data::user::User;
use iced::widget::{column, container, row, text};
use iced::Length;
use once_cell::sync::Lazy;

use crate::theme;
use crate::widget::{double_pass, Element};

const MAX_SHOWN_ENTRIES: usize = 5;

#[derive(Debug, Clone, Default)]
pub struct Completion {
    commands: Commands,
    text: Text,
}

impl Completion {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Process input and update the completion state
    pub fn process(&mut self, input: &str, users: &[User], channels: &[String]) {
        let is_command = input.starts_with('/');

        if is_command {
            self.commands.process(input);

            // Disallow user completions when selecting a command
            if matches!(self.commands, Commands::Selecting { .. }) {
                self.text = Text::default();
            } else {
                self.text.process(input, users, channels);
            }
        } else {
            self.text.process(input, users, channels);
            self.commands = Commands::default();
        }
    }

    pub fn select(&mut self) -> Option<Entry> {
        self.commands.select().map(Entry::Command)
    }

    pub fn tab(&mut self) -> Option<Entry> {
        if !self.commands.tab() {
            self.text.tab().map(Entry::Text)
        } else {
            None
        }
    }

    pub fn view<'a, Message: 'a>(&self, input: &str) -> Option<Element<'a, Message>> {
        self.commands.view(input)
    }
}

#[derive(Debug, Clone)]
pub enum Entry {
    Command(Command),
    Text(String),
}

impl Entry {
    pub fn complete_input(&self, input: &str) -> String {
        match self {
            Entry::Command(command) => format!("/{}", command.title),
            Entry::Text(value) => match input.rsplit_once(' ') {
                Some((left, _)) => format!("{left} {value}"),
                None => value.clone(),
            },
        }
    }
}

#[derive(Debug, Clone)]
enum Commands {
    Idle,
    Selecting {
        highlighted: Option<usize>,
        filtered: Vec<Command>,
    },
    Selected {
        command: Command,
    },
}

impl Default for Commands {
    fn default() -> Self {
        Self::Idle
    }
}

impl Commands {
    fn process(&mut self, input: &str) {
        let Some((head, rest)) = input.split_once('/') else {
            *self = Self::Idle;
            return;
        };

        // Don't allow text before a command slash
        if !head.is_empty() {
            *self = Self::Idle;
            return;
        }

        let (cmd, has_space) = if let Some(index) = rest.find(' ') {
            (&rest[0..index], true)
        } else {
            (rest, false)
        };

        match self {
            // Command not fully typed, show filtered entries
            _ if !has_space => {
                let filtered = COMMAND_LIST
                    .iter()
                    .filter(|command| {
                        command
                            .title
                            .to_lowercase()
                            .starts_with(&cmd.to_lowercase())
                    })
                    .cloned()
                    .collect();

                *self = Self::Selecting {
                    highlighted: None,
                    filtered,
                };
            }
            // Command fully typed, transition to showing known entry
            Self::Idle | Self::Selecting { .. } => {
                if let Some(command) = COMMAND_LIST
                    .iter()
                    .find(|command| command.title.to_lowercase() == cmd.to_lowercase())
                    .cloned()
                {
                    *self = Self::Selected { command };
                } else {
                    *self = Self::Idle
                }
            }
            // Command fully typed & already selected, do nothing
            Self::Selected { .. } => {}
        }
    }

    fn select(&mut self) -> Option<Command> {
        if let Self::Selecting {
            highlighted: Some(index),
            filtered,
        } = self
        {
            if let Some(command) = filtered.get(*index).cloned() {
                *self = Self::Selected {
                    command: command.clone(),
                };

                return Some(command);
            }
        }

        None
    }

    fn tab(&mut self) -> bool {
        if let Self::Selecting {
            highlighted,
            filtered,
        } = self
        {
            if filtered.is_empty() {
                *highlighted = None;
            } else if let Some(index) = highlighted {
                *index = (*index + 1) % filtered.len();
            } else {
                *highlighted = Some(0);
            }

            true
        } else {
            false
        }
    }

    fn view<'a, Message: 'a>(&self, input: &str) -> Option<Element<'a, Message>> {
        match self {
            Self::Idle => None,
            Self::Selecting {
                highlighted,
                filtered,
            } => {
                let skip = {
                    let index = if let Some(index) = highlighted {
                        *index
                    } else {
                        0
                    };

                    let to = index.max(MAX_SHOWN_ENTRIES - 1);
                    to.saturating_sub(MAX_SHOWN_ENTRIES - 1)
                };

                let entries = filtered
                    .iter()
                    .enumerate()
                    .skip(skip)
                    .take(MAX_SHOWN_ENTRIES)
                    .collect::<Vec<_>>();

                let content = |width| {
                    column(entries.iter().map(|(index, command)| {
                        let selected = Some(*index) == *highlighted;
                        let content = text(format!("/{}", command.title));

                        Element::from(
                            container(content)
                                .width(width)
                                .style(theme::Container::Command { selected })
                                .padding(6)
                                .center_y(),
                        )
                    }))
                };

                (!entries.is_empty()).then(|| {
                    let first_pass = content(Length::Shrink);
                    let second_pass = content(Length::Fill);

                    container(double_pass(first_pass, second_pass))
                        .padding(4)
                        .style(theme::Container::Context)
                        .into()
                })
            }
            Self::Selected { command } => Some(command.view(input)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    title: &'static str,
    args: Vec<Arg>,
}

impl Command {
    fn view<'a, Message: 'a>(&self, input: &str) -> Element<'a, Message> {
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

        container(row(title.into_iter().chain(args)))
            .style(theme::Container::Context)
            .padding(8)
            .center_y()
            .into()
    }
}

#[derive(Debug, Clone)]
struct Arg {
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

#[derive(Debug, Clone, Default)]
struct Text {
    prompt: String,
    filtered: Vec<String>,
    selected: Option<usize>,
}

impl Text {
    fn process(&mut self, input: &str, users: &[User], channels: &[String]) {
        if !self.process_channels(input, channels) {
            self.process_users(input, users);
        }
    }

    fn process_users(&mut self, input: &str, users: &[User]) {
        let (_, rest) = input.rsplit_once(' ').unwrap_or(("", input));

        if rest.is_empty() {
            *self = Self::default();
            return;
        }

        let nick = rest.to_lowercase();

        self.selected = None;
        self.prompt = rest.to_string();
        self.filtered = users
            .iter()
            .filter_map(|user| {
                let lower_nick = user.nickname().as_ref().to_lowercase();
                lower_nick
                    .starts_with(&nick)
                    .then(|| user.nickname().to_string())
            })
            .collect();
    }

    fn process_channels(&mut self, input: &str, channels: &[String]) -> bool {
        let (_, last) = input.rsplit_once(' ').unwrap_or(("", input));
        let Some((_, rest)) = last.split_once('#') else {
            *self = Self::default();
            return false;
        };

        let channel = format!("#{}", rest.to_lowercase());

        self.selected = None;
        self.prompt = format!("#{rest}");
        self.filtered = channels
            .iter()
            .filter_map(|c| {
                let lower_channel = c.to_lowercase();
                lower_channel.starts_with(&channel).then(|| c.to_string())
            })
            .collect();

        true
    }

    fn tab(&mut self) -> Option<String> {
        if !self.filtered.is_empty() {
            if let Some(index) = &mut self.selected {
                if *index < self.filtered.len() - 1 {
                    *index += 1;
                } else {
                    self.selected = None;
                }
            } else {
                self.selected = Some(0);
            }
        }

        if let Some(index) = self.selected {
            self.filtered.get(index).cloned()
        } else {
            (!self.prompt.is_empty()).then(|| self.prompt.clone())
        }
    }
}

static COMMAND_LIST: Lazy<Vec<Command>> = Lazy::new(|| {
    vec![
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
    ]
});
