use std::fmt;

use data::isupport;
use data::user::User;
use iced::widget::{column, container, row, text, tooltip};
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
    pub fn process(
        &mut self,
        input: &str,
        users: &[User],
        channels: &[String],
        isupport_parameters: &[&isupport::Parameter],
    ) {
        let is_command = input.starts_with('/');

        if is_command {
            self.commands.process(input, isupport_parameters);

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
    fn process(&mut self, input: &str, isupport_parameters: &[&isupport::Parameter]) {
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

        let command_list = COMMAND_LIST
            .iter()
            .map(|command| {
                if command.title == "WHO"
                    && find_isupport_parameter(isupport_parameters, isupport::Kind::WHOX).is_some()
                {
                    return WHOX_COMMAND.clone();
                } else if command.title == "NICK" {
                    if let Some(isupport::Parameter::NICKLEN(nick_len)) =
                        find_isupport_parameter(isupport_parameters, isupport::Kind::NICKLEN)
                    {
                        return nick_command(nick_len);
                    }
                }

                command.clone()
            })
            .chain(
                isupport_parameters.iter().filter_map(|isupport_parameter| {
                    isupport_parameter_to_command(isupport_parameter)
                }),
            )
            .collect::<Vec<_>>();

        match self {
            // Command not fully typed, show filtered entries
            _ if !has_space => {
                let filtered = command_list
                    .into_iter()
                    .filter(|command| {
                        command
                            .title
                            .to_lowercase()
                            .starts_with(&cmd.to_lowercase())
                    })
                    .collect();

                *self = Self::Selecting {
                    highlighted: None,
                    filtered,
                };
            }
            // Command fully typed, transition to showing known entry
            Self::Idle | Self::Selecting { .. } => {
                if let Some(command) = command_list
                    .into_iter()
                    .find(|command| command.title.to_lowercase() == cmd.to_lowercase())
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
                                .style(if selected {
                                    theme::container::command_selected
                                } else {
                                    theme::container::command
                                })
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
                        .style(theme::container::context)
                        .width(Length::Shrink)
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
            let content = text(format!(" {arg}")).style(move |theme| {
                if index == active_arg {
                    theme::text::accent(theme)
                } else {
                    theme::text::none(theme)
                }
            });

            if let Some(arg_tooltip) = &arg.tooltip {
                Element::from(tooltip(
                    content,
                    container(text(arg_tooltip.clone()).style(move |theme| {
                        if index == active_arg {
                            theme::text::transparent_accent(theme)
                        } else {
                            theme::text::transparent(theme)
                        }
                    }))
                    .style(theme::container::context)
                    .padding(8),
                    tooltip::Position::Top,
                ))
            } else {
                Element::from(content)
            }
        });

        container(row(title.into_iter().chain(args)))
            .style(theme::container::context)
            .padding(8)
            .center_y()
            .into()
    }
}

#[derive(Debug, Clone)]
struct Arg {
    text: &'static str,
    optional: bool,
    tooltip: Option<String>,
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
                    tooltip: None,
                },
                Arg {
                    text: "keys",
                    optional: true,
                    tooltip: None,
                },
            ],
        },
        Command {
            title: "MOTD",
            args: vec![Arg {
                text: "server",
                optional: true,
                tooltip: None,
            }],
        },
        Command {
            title: "NICK",
            args: vec![Arg {
                text: "nickname",
                optional: false,
                tooltip: None,
            }],
        },
        Command {
            title: "QUIT",
            args: vec![Arg {
                text: "reason",
                optional: true,
                tooltip: None,
            }],
        },
        Command {
            title: "MSG",
            args: vec![
                Arg {
                    text: "target",
                    optional: false,
                    tooltip: None,
                },
                Arg {
                    text: "text",
                    optional: false,
                    tooltip: None,
                },
            ],
        },
        Command {
            title: "WHOIS",
            args: vec![Arg {
                text: "nick",
                optional: false,
                tooltip: None,
            }],
        },
        Command {
            title: "ME",
            args: vec![Arg {
                text: "action",
                optional: false,
                tooltip: None,
            }],
        },
        Command {
            title: "MODE",
            args: vec![
                Arg {
                    text: "channel",
                    optional: false,
                    tooltip: None,
                },
                Arg {
                    text: "mode",
                    optional: false,
                    tooltip: None,
                },
                Arg {
                    text: "user",
                    optional: true,
                    tooltip: None,
                },
            ],
        },
        Command {
            title: "PART",
            args: vec![
                Arg {
                    text: "channels",
                    optional: false,
                    tooltip: None,
                },
                Arg {
                    text: "reason",
                    optional: true,
                    tooltip: None,
                },
            ],
        },
        Command {
            title: "TOPIC",
            args: vec![
                Arg {
                    text: "channel",
                    optional: false,
                    tooltip: None,
                },
                Arg {
                    text: "topic",
                    optional: true,
                    tooltip: None,
                },
            ],
        },
        Command {
            title: "WHO",
            args: vec![Arg {
                text: "target",
                optional: false,
                tooltip: None,
            }],
        },
        Command {
            title: "KICK",
            args: vec![
                Arg {
                    text: "channel",
                    optional: false,
                    tooltip: None,
                },
                Arg {
                    text: "user",
                    optional: false,
                    tooltip: None,
                },
                Arg {
                    text: "comment",
                    optional: true,
                    tooltip: None,
                },
            ],
        },
        Command {
            title: "RAW",
            args: vec![
                Arg {
                    text: "command",
                    optional: false,
                    tooltip: None,
                },
                Arg {
                    text: "args",
                    optional: true,
                    tooltip: None,
                },
            ],
        },
    ]
});

fn find_isupport_parameter(
    isupport_parameters: &[&isupport::Parameter],
    kind: isupport::Kind,
) -> Option<isupport::Parameter> {
    isupport_parameters
        .iter()
        .find(|isupport_parameter| isupport_parameter.kind() == Some(kind.clone()))
        .cloned()
        .cloned()
}

fn isupport_parameter_to_command(isupport_parameter: &isupport::Parameter) -> Option<Command> {
    match isupport_parameter {
        isupport::Parameter::KNOCK => Some(KNOCK_COMMAND.clone()),
        isupport::Parameter::USERIP => Some(USERIP_COMMAND.clone()),
        isupport::Parameter::CNOTICE => Some(CNOTICE_COMMAND.clone()),
        isupport::Parameter::CPRIVMSG => Some(CPRIVMSG_COMMAND.clone()),
        isupport::Parameter::SAFELIST => Some(LIST_COMMAND.clone()),
        _ => None,
    }
}

static CNOTICE_COMMAND: Lazy<Command> = Lazy::new(|| Command {
    title: "CNOTICE",
    args: vec![
        Arg {
            text: "nickname",
            optional: false,
            tooltip: None,
        },
        Arg {
            text: "channel",
            optional: false,
            tooltip: None,
        },
        Arg {
            text: "message",
            optional: false,
            tooltip: None,
        },
    ],
});

static CPRIVMSG_COMMAND: Lazy<Command> = Lazy::new(|| Command {
    title: "CPRIVMSG",
    args: vec![
        Arg {
            text: "nickname",
            optional: false,
            tooltip: None,
        },
        Arg {
            text: "channel",
            optional: false,
            tooltip: None,
        },
        Arg {
            text: "message",
            optional: false,
            tooltip: None,
        },
    ],
});

static KNOCK_COMMAND: Lazy<Command> = Lazy::new(|| Command {
    title: "KNOCK",
    args: vec![
        Arg {
            text: "channel",
            optional: false,
            tooltip: None,
        },
        Arg {
            text: "message",
            optional: true,
            tooltip: None,
        },
    ],
});

static LIST_COMMAND: Lazy<Command> = Lazy::new(|| Command {
    title: "LIST",
    args: vec![
        Arg {
            text: "channels",
            optional: true,
            tooltip: None,
        },
        Arg {
            text: "server",
            optional: true,
            tooltip: None,
        },
    ],
});

fn nick_command(max_len: u16) -> Command {
    Command {
        title: "NICK",
        args: vec![Arg {
            text: "nickname",
            optional: false,
            tooltip: Some(format!("maximum length: {}", max_len)),
        }],
    }
}

static USERIP_COMMAND: Lazy<Command> = Lazy::new(|| Command {
    title: "USERIP",
    args: vec![Arg {
        text: "nickname",
        optional: false,
        tooltip: None,
    }],
});

static WHOX_COMMAND: Lazy<Command> = Lazy::new(|| Command {
    title: "WHO",
    args: vec![
        Arg {
            text: "target",
            optional: false,
            tooltip: None,
        },
        Arg {
            text: "fields",
            optional: true,
            tooltip: Some(String::from(
                "t: token\n\
                c: channel\n\
            u: username\n\
            i: IP address\n\
            h: hostname\n\
            s: server name\n\
            n: nickname\n\
            f: WHO flags\n\
            d: hop count\n\
            l: idle seconds\n\
            a: account name\n\
            o: channel op level\n\
            r: realname",
            )),
        },
        Arg {
            text: "token",
            optional: true,
            tooltip: Some(String::from("1-3 digits")),
        },
    ],
});
