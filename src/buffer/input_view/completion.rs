use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

use data::buffer::{SkinTone, SortDirection};
use data::isupport::{self, find_target_limit};
use data::user::User;
use data::{Config, target};
use iced::Length;
use iced::widget::{column, container, row, text, tooltip};
use itertools::Itertools;
use strsim::jaro_winkler;

use crate::theme;
use crate::widget::{Element, double_pass};

const MAX_SHOWN_COMMAND_ENTRIES: usize = 5;
const MAX_SHOWN_EMOJI_ENTRIES: usize = 8;

#[derive(Debug, Clone, Default)]
pub struct Completion {
    commands: Commands,
    text: Text,
    emojis: Emojis,
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
        channels: &[target::Channel],
        isupport: &HashMap<isupport::Kind, isupport::Parameter>,
        config: &Config,
    ) {
        let is_command = input.starts_with('/');

        let casemapping =
            if let Some(isupport::Parameter::CASEMAPPING(casemapping)) =
                isupport.get(&isupport::Kind::CASEMAPPING)
            {
                *casemapping
            } else {
                isupport::CaseMap::default()
            };

        if is_command {
            self.commands.process(input, isupport);

            // Disallow user completions when selecting a command
            if matches!(self.commands, Commands::Selecting { .. }) {
                self.text = Text::default();
            } else {
                self.text
                    .process(input, casemapping, users, channels, config);
            }

            self.emojis = Emojis::default();
        } else if let Some(shortcode) = (config.buffer.emojis.show_picker
            || config.buffer.emojis.auto_replace)
            .then(|| {
                input
                    .split(' ')
                    .next_back()
                    .filter(|last_word| last_word.starts_with(':'))
            })
            .flatten()
        {
            self.emojis.process(shortcode, config);

            self.commands = Commands::default();
            self.text = Text::default();
        } else {
            self.text
                .process(input, casemapping, users, channels, config);

            self.commands = Commands::default();

            self.emojis = Emojis::default();
        }
    }

    pub fn select(&mut self, config: &Config) -> Option<Entry> {
        self.commands
            .select()
            .map(Entry::Command)
            .or(self.emojis.select(config).map(Entry::Emoji))
    }

    pub fn complete_emoji(&self, input: &str) -> Option<String> {
        if let Emojis::Selected { emoji } = self.emojis {
            Some(replace_last_word_with_emoji(input, emoji))
        } else {
            None
        }
    }

    pub fn tab(&mut self, reverse: bool) -> Option<Entry> {
        if self.commands.tab(reverse) {
            return None;
        }

        if self.emojis.tab(reverse) {
            return None;
        }

        self.text.tab(reverse).map_or(
            {
                if self.text.filtered.is_empty() {
                    None
                } else {
                    Some(Entry::Text {
                        next: self.text.prompt.clone(),
                        append_suffix: false,
                    })
                }
            },
            |next| {
                Some(Entry::Text {
                    next,
                    append_suffix: true,
                })
            },
        )
    }

    pub fn arrow(&mut self, arrow: Arrow) -> bool {
        let reverse = match arrow {
            Arrow::Up => true,
            Arrow::Down => false,
        };

        if self.commands.tab(reverse) {
            return true;
        }

        if self.emojis.tab(reverse) {
            return true;
        }

        false
    }

    pub fn view<'a, Message: 'a>(
        &self,
        input: &str,
        config: &Config,
    ) -> Option<Element<'a, Message>> {
        self.commands
            .view(input, config)
            .or(self.emojis.view(config))
    }

    pub fn close_picker(&mut self) -> bool {
        if matches!(self.commands, Commands::Selecting { .. }) {
            self.commands = Commands::Idle;

            return true;
        } else if matches!(self.emojis, Emojis::Selecting { .. }) {
            self.emojis = Emojis::Idle;

            return true;
        }

        false
    }
}

#[derive(Debug, Clone)]
pub enum Entry {
    Command(Command),
    Text { next: String, append_suffix: bool },
    Emoji(String),
}

impl Entry {
    pub fn complete_input(
        &self,
        input: &str,
        chantypes: &[char],
        config: &Config,
    ) -> String {
        match self {
            Entry::Command(command) => {
                format!("/{}", command.title.to_lowercase())
            }
            Entry::Text {
                next,
                append_suffix,
            } => {
                let autocomplete = &config.buffer.text_input.autocomplete;
                let is_channel = next.starts_with(chantypes);
                let mut words: Vec<_> = input.split(' ').collect();

                if let Some(last_word_position) =
                    words.iter().rposition(|word| !word.is_empty())
                {
                    words.truncate(last_word_position + 1);
                } else {
                    words.clear();
                }

                // Replace the last word with the next word
                if let Some(last_word) = words.last_mut() {
                    *last_word = next;
                } else {
                    words.push(next);
                }

                let mut new_input = words.join(" ");

                // If next is not the original prompt, then append the configured suffix
                if *append_suffix {
                    if words.len() == 1 && !is_channel {
                        // If completed at the beginning of the input line and not a channel.
                        let suffix = &autocomplete.completion_suffixes[0];
                        new_input.push_str(suffix);
                    } else {
                        // Otherwise, use second suffix.
                        let suffix = &autocomplete.completion_suffixes[1];
                        new_input.push_str(suffix);
                    }
                }

                new_input
            }
            Entry::Emoji(emoji) => replace_last_word_with_emoji(input, emoji),
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
        subcommand: Option<Command>,
    },
}

impl Default for Commands {
    fn default() -> Self {
        Self::Idle
    }
}

impl Commands {
    fn process(
        &mut self,
        input: &str,
        isupport: &HashMap<isupport::Kind, isupport::Parameter>,
    ) {
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
                match command.title {
                    "AWAY" => {
                        if let Some(isupport::Parameter::AWAYLEN(max_len)) =
                            isupport.get(&isupport::Kind::AWAYLEN)
                        {
                            return away_command(max_len);
                        }
                    }
                    "JOIN" => {
                        let channel_len = if let Some(
                            isupport::Parameter::CHANNELLEN(max_len),
                        ) =
                            isupport.get(&isupport::Kind::CHANNELLEN)
                        {
                            Some(max_len)
                        } else {
                            None
                        };

                        let channel_limits = if let Some(
                            isupport::Parameter::CHANLIMIT(channel_limits),
                        ) =
                            isupport.get(&isupport::Kind::CHANLIMIT)
                        {
                            Some(channel_limits)
                        } else {
                            None
                        };

                        let key_len =
                            if let Some(isupport::Parameter::KEYLEN(max_len)) =
                                isupport.get(&isupport::Kind::KEYLEN)
                            {
                                Some(max_len)
                            } else {
                                None
                            };

                        if channel_len.is_some()
                            || channel_limits.is_some()
                            || key_len.is_some()
                        {
                            return join_command(
                                channel_len,
                                channel_limits,
                                key_len,
                            );
                        }
                    }
                    "KICK" => {
                        let kick_len = if let Some(
                            isupport::Parameter::KICKLEN(max_len),
                        ) =
                            isupport.get(&isupport::Kind::KICKLEN)
                        {
                            Some(max_len)
                        } else {
                            None
                        };

                        let target_limit = find_target_limit(isupport, "KICK");

                        if target_limit.is_some() || kick_len.is_some() {
                            return kick_command(target_limit, kick_len);
                        }
                    }
                    "MSG" => {
                        let channel_membership_prefixes =
                            if let Some(isupport::Parameter::STATUSMSG(
                                channel_membership_prefixes,
                            )) = isupport.get(&isupport::Kind::STATUSMSG)
                            {
                                channel_membership_prefixes.clone()
                            } else {
                                vec![]
                            };

                        let target_limit =
                            find_target_limit(isupport, "PRIVMSG");

                        if !channel_membership_prefixes.is_empty()
                            || target_limit.is_some()
                        {
                            return msg_command(
                                channel_membership_prefixes,
                                target_limit,
                            );
                        }
                    }
                    "NAMES" => {
                        if let Some(target_limit) =
                            find_target_limit(isupport, command.title)
                        {
                            return names_command(target_limit);
                        }
                    }
                    "NICK" => {
                        if let Some(isupport::Parameter::NICKLEN(max_len)) =
                            isupport.get(&isupport::Kind::NICKLEN)
                        {
                            return nick_command(max_len);
                        }
                    }
                    "NOTICE" => {
                        let channel_membership_prefixes =
                            if let Some(isupport::Parameter::STATUSMSG(
                                channel_membership_prefixes,
                            )) = isupport.get(&isupport::Kind::STATUSMSG)
                            {
                                channel_membership_prefixes.clone()
                            } else {
                                vec![]
                            };

                        let target_limit =
                            find_target_limit(isupport, "NOTICE");

                        if !channel_membership_prefixes.is_empty()
                            || target_limit.is_some()
                        {
                            return notice_command(
                                channel_membership_prefixes,
                                target_limit,
                            );
                        }
                    }
                    "PART" => {
                        if let Some(isupport::Parameter::CHANNELLEN(max_len)) =
                            isupport.get(&isupport::Kind::CHANNELLEN)
                        {
                            return part_command(max_len);
                        }
                    }
                    "TOPIC" => {
                        if let Some(isupport::Parameter::TOPICLEN(max_len)) =
                            isupport.get(&isupport::Kind::TOPICLEN)
                        {
                            return topic_command(max_len);
                        }
                    }
                    "WHO" => {
                        if isupport.get(&isupport::Kind::WHOX).is_some() {
                            return WHOX_COMMAND.clone();
                        }
                    }
                    "WHOIS" => {
                        if let Some(target_limit) =
                            find_target_limit(isupport, command.title)
                        {
                            return whois_command(target_limit);
                        }
                    }
                    _ => (),
                }

                command.clone()
            })
            .chain(isupport.iter().filter_map(|(_, isupport_parameter)| {
                match isupport_parameter {
                    isupport::Parameter::CHATHISTORY(maximum_limit) => {
                        Some(chathistory_command(maximum_limit))
                    }
                    isupport::Parameter::MONITOR(target_limit) => {
                        Some(monitor_command(target_limit))
                    }
                    isupport::Parameter::SAFELIST => {
                        let search_extensions = if let Some(
                            isupport::Parameter::ELIST(search_extensions),
                        ) =
                            isupport.get(&isupport::Kind::ELIST)
                        {
                            Some(search_extensions)
                        } else {
                            None
                        };

                        let target_limit = find_target_limit(isupport, "LIST");

                        if search_extensions.is_some() || target_limit.is_some()
                        {
                            Some(list_command(search_extensions, target_limit))
                        } else {
                            Some(LIST_COMMAND.clone())
                        }
                    }
                    isupport::Parameter::NAMELEN(max_len) => {
                        Some(setname_command(max_len))
                    }
                    _ => isupport_parameter_to_command(isupport_parameter),
                }
            }))
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
                    highlighted: Some(0),
                    filtered,
                };
            }
            // Command fully typed, transition to showing known entry
            Self::Idle | Self::Selecting { .. } => {
                if let Some(command) =
                    command_list.into_iter().find(|command| {
                        command.title.to_lowercase() == cmd.to_lowercase()
                            || command.alias().iter().any(|alias| {
                                alias.to_lowercase() == cmd.to_lowercase()
                            })
                    })
                {
                    *self = Self::Selected {
                        command,
                        subcommand: None,
                    };
                } else {
                    *self = Self::Idle;
                }
            }
            // Command fully typed & already selected, check for subcommand if any exist
            Self::Selected { command, .. } => {
                if let Some(subcommands) = &command.subcommands {
                    let subcmd =
                        if let Some(index) = &rest[cmd.len() + 1..].find(' ') {
                            &rest[0..cmd.len() + 1 + index]
                        } else {
                            rest
                        };

                    let subcommand = subcommands.iter().find(|subcommand| {
                        subcommand.title.to_lowercase() == subcmd.to_lowercase()
                            || subcommand.alias().iter().any(|alias| {
                                alias.to_lowercase() == subcmd.to_lowercase()
                            })
                    });

                    *self = Self::Selected {
                        command: command.clone(),
                        subcommand: subcommand.cloned(),
                    };
                }
            }
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
                    subcommand: None,
                };

                return Some(command);
            }
        }

        None
    }

    fn tab(&mut self, reverse: bool) -> bool {
        if let Self::Selecting {
            highlighted,
            filtered,
        } = self
        {
            selecting_tab(highlighted, filtered, reverse);

            true
        } else {
            false
        }
    }

    fn view<'a, Message: 'a>(
        &self,
        input: &str,
        config: &Config,
    ) -> Option<Element<'a, Message>> {
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

                    let to = index.max(MAX_SHOWN_COMMAND_ENTRIES - 1);
                    to.saturating_sub(MAX_SHOWN_COMMAND_ENTRIES - 1)
                };

                let entries = filtered
                    .iter()
                    .enumerate()
                    .skip(skip)
                    .take(MAX_SHOWN_COMMAND_ENTRIES)
                    .collect::<Vec<_>>();

                let content = |width| {
                    column(entries.iter().map(|(index, command)| {
                        let selected = Some(*index) == *highlighted;
                        let content =
                            text(format!("/{}", command.title.to_lowercase()));

                        Element::from(
                            container(content)
                                .width(width)
                                .style(if selected {
                                    theme::container::primary_background_hover
                                } else {
                                    theme::container::none
                                })
                                .padding(6)
                                .center_y(Length::Shrink),
                        )
                    }))
                };

                (!entries.is_empty()).then(|| {
                    let first_pass = content(Length::Shrink);
                    let second_pass = content(Length::Fill);

                    container(double_pass(first_pass, second_pass))
                        .padding(4)
                        .style(theme::container::tooltip)
                        .width(Length::Shrink)
                        .into()
                })
            }
            Self::Selected {
                command,
                subcommand,
            } => {
                if config.buffer.commands.show_description {
                    subcommand
                        .as_ref()
                        .map(|sub| sub.view(input))
                        .or_else(|| Some(command.view(input)))
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    title: &'static str,
    args: Vec<Arg>,
    subcommands: Option<Vec<Command>>,
}

impl Command {
    fn description(&self) -> Option<&'static str> {
        Some(match self.title.to_lowercase().as_str() {
            "away" => {
                "Mark yourself as away. If already away, the status is removed"
            }
            "join" => "Join channel(s) with optional key(s)",
            "me" => "Send an action message to the channel",
            "mode" => {
                "Set mode(s) on a target or retrieve the current mode(s) set. A target can be a channel or an user"
            }
            "monitor" => "System to notify when users become online/offline",
            "monitor +" => "Add user(s) to list being monitored",
            "monitor -" => "Remove user(s) from list being monitored",
            "monitor c" => "Clear the list of users being monitored",
            "monitor l" => "Get list of users being monitored",
            "monitor s" => {
                "For each user in the list being monitored, get the current status"
            }
            "msg" => {
                "Open a query with a nickname and send an optional message"
            }
            "nick" => "Change your nickname on the current server",
            "part" => "Leave channel(s) with an optional reason",
            "quit" => "Disconnect from the server with an optional reason",
            "raw" => "Send data to the server without modifying it",
            "topic" => "Retrieve the topic of a channel or set a new topic",
            "whois" => "Retrieve information about user(s)",
            "format" => "Format text using markdown or $ sequences",
            "ctcp" => "Send Client-To-Client requests",

            _ => return None,
        })
    }

    fn alias(&self) -> Vec<&str> {
        match self.title.to_lowercase().as_str() {
            "away" => vec![],
            "join" => vec!["j"],
            "me" => vec!["describe"],
            "mode" => vec!["m"],
            "msg" => vec![],
            "nick" => vec![],
            "part" => vec!["leave"],
            "quit" => vec![""],
            "raw" => vec![],
            "topic" => vec!["t"],
            "whois" => vec![],
            "format" => vec!["f"],

            _ => vec![],
        }
    }

    fn view<'a, Message: 'a>(&self, input: &str) -> Element<'a, Message> {
        let command_prefix = format!("/{}", self.title.to_lowercase());

        let active_arg = [
            "_",
            input
                .to_lowercase()
                .strip_prefix(&command_prefix)
                .unwrap_or(input),
            "_",
        ]
        .concat()
        .split_ascii_whitespace()
        .count()
        .saturating_sub(2)
        .min(self.args.len().saturating_sub(1));

        let title = Some(Element::from(text(self.title)));

        let args = self.args.iter().enumerate().map(|(index, arg)| {
            let content = text(format!("{arg}")).style(move |theme| {
                if index == active_arg {
                    theme::text::tertiary(theme)
                } else {
                    theme::text::none(theme)
                }
            });

            if let Some(arg_tooltip) = &arg.tooltip {
                let tooltip_indicator = text("*")
                    .style(move |theme| {
                        if index == active_arg {
                            theme::text::tertiary(theme)
                        } else {
                            theme::text::none(theme)
                        }
                    })
                    .size(8);

                Element::from(row![
                    text(" "),
                    tooltip(
                        row![content, tooltip_indicator]
                            .align_y(iced::Alignment::Start),
                        container(text(arg_tooltip.clone()).style(
                            move |theme| {
                                if index == active_arg {
                                    theme::text::tertiary(theme)
                                } else {
                                    theme::text::secondary(theme)
                                }
                            }
                        ))
                        .style(theme::container::tooltip)
                        .padding(8),
                        tooltip::Position::Top,
                    )
                ])
            } else {
                Element::from(row![text(" "), content])
            }
        });

        container(
            column![]
                .push_maybe(self.description().map(|description| {
                    text(description).style(theme::text::secondary)
                }))
                .push(row(title.into_iter().chain(args))),
        )
        .style(theme::container::tooltip)
        .padding(8)
        .center_y(Length::Shrink)
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
    fn process(
        &mut self,
        input: &str,
        casemapping: isupport::CaseMap,
        users: &[User],
        channels: &[target::Channel],
        config: &Config,
    ) {
        if !self.process_channels(input, casemapping, channels, config) {
            self.process_users(input, users, config);
        }
    }

    fn process_users(&mut self, input: &str, users: &[User], config: &Config) {
        let autocomplete = &config.buffer.text_input.autocomplete;
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
            .sorted_by(|a, b| match autocomplete.sort_direction {
                SortDirection::Asc => a.nickname().cmp(&b.nickname()),
                SortDirection::Desc => b.nickname().cmp(&a.nickname()),
            })
            .filter_map(|user| {
                let lower_nick = user.nickname().as_ref().to_lowercase();
                lower_nick
                    .starts_with(&nick)
                    .then(|| user.nickname().to_string())
            })
            .collect();
    }

    fn process_channels(
        &mut self,
        input: &str,
        casemapping: isupport::CaseMap,
        channels: &[target::Channel],
        config: &Config,
    ) -> bool {
        let autocomplete = &config.buffer.text_input.autocomplete;
        let (_, last) = input.rsplit_once(' ').unwrap_or(("", input));
        let Some((_, rest)) = last.split_once('#') else {
            *self = Self::default();
            return false;
        };

        let input_channel = format!("#{}", casemapping.normalize(rest));

        self.selected = None;
        self.prompt = format!("#{rest}");
        self.filtered = channels
            .iter()
            .sorted_by(|a, b: &&target::Channel| {
                match autocomplete.sort_direction {
                    SortDirection::Asc => {
                        a.as_normalized_str().cmp(b.as_normalized_str())
                    }
                    SortDirection::Desc => {
                        b.as_normalized_str().cmp(a.as_normalized_str())
                    }
                }
            })
            .filter(|&channel| {
                channel.as_str().starts_with(input_channel.as_str())
            })
            .map(|channel| channel.to_string())
            .collect();

        true
    }

    fn tab(&mut self, reverse: bool) -> Option<String> {
        if !self.filtered.is_empty() {
            if let Some(index) = &mut self.selected {
                if reverse {
                    if *index > 0 {
                        *index -= 1;
                    } else {
                        self.selected = None;
                    }
                } else if *index < self.filtered.len() - 1 {
                    *index += 1;
                } else {
                    self.selected = None;
                }
            } else {
                self.selected =
                    Some(if reverse { self.filtered.len() - 1 } else { 0 });
            }
        }

        if let Some(index) = self.selected {
            self.filtered.get(index).cloned()
        } else {
            None
        }
    }
}

static COMMAND_LIST: LazyLock<Vec<Command>> = LazyLock::new(|| {
    vec![
        Command {
            title: "JOIN",
            args: vec![
                Arg {
                    text: "channels",
                    optional: false,
                    tooltip: Some(String::from("comma-separated")),
                },
                Arg {
                    text: "keys",
                    optional: true,
                    tooltip: Some(String::from("comma-separated")),
                },
            ],
            subcommands: None,
        },
        Command {
            title: "MOTD",
            args: vec![Arg {
                text: "server",
                optional: true,
                tooltip: None,
            }],
            subcommands: None,
        },
        Command {
            title: "NICK",
            args: vec![Arg {
                text: "nickname",
                optional: false,
                tooltip: None,
            }],
            subcommands: None,
        },
        Command {
            title: "QUIT",
            args: vec![Arg {
                text: "reason",
                optional: true,
                tooltip: None,
            }],
            subcommands: None,
        },
        Command {
            title: "MSG",
            args: vec![
                Arg {
                    text: "targets",
                    optional: false,
                    tooltip: Some(String::from(
                        "comma-separated\n   {user}: user directly\n{channel}: all users in channel",
                    )),
                },
                Arg {
                    text: "text",
                    optional: true,
                    tooltip: None,
                },
            ],
            subcommands: None,
        },
        Command {
            title: "NOTICE",
            args: vec![
                Arg {
                    text: "targets",
                    optional: false,
                    tooltip: Some(String::from(
                        "comma-separated\n   {user}: user directly\n{channel}: all users in channel",
                    )),
                },
                Arg {
                    text: "text",
                    optional: true,
                    tooltip: None,
                },
            ],
            subcommands: None,
        },
        Command {
            title: "WHOIS",
            args: vec![Arg {
                text: "nicks",
                optional: false,
                tooltip: Some(String::from("comma-separated")),
            }],
            subcommands: None,
        },
        Command {
            title: "AWAY",
            args: vec![Arg {
                text: "reason",
                optional: true,
                tooltip: None,
            }],
            subcommands: None,
        },
        Command {
            title: "ME",
            args: vec![Arg {
                text: "action",
                optional: false,
                tooltip: None,
            }],
            subcommands: None,
        },
        Command {
            title: "MODE",
            args: vec![
                Arg {
                    text: "target",
                    optional: false,
                    tooltip: None,
                },
                Arg {
                    text: "modestring",
                    optional: true,
                    tooltip: None,
                },
                Arg {
                    text: "arguments",
                    optional: true,
                    tooltip: None,
                },
            ],
            subcommands: None,
        },
        Command {
            title: "PART",
            args: vec![
                Arg {
                    text: "channels",
                    optional: false,
                    tooltip: Some(String::from("comma-separated")),
                },
                Arg {
                    text: "reason",
                    optional: true,
                    tooltip: None,
                },
            ],
            subcommands: None,
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
            subcommands: None,
        },
        Command {
            title: "WHO",
            args: vec![Arg {
                text: "target",
                optional: false,
                tooltip: None,
            }],
            subcommands: None,
        },
        Command {
            title: "NAMES",
            args: vec![Arg {
                text: "channels",
                optional: false,
                tooltip: Some(String::from("comma-separated")),
            }],
            subcommands: None,
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
            subcommands: None,
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
            subcommands: None,
        },
        Command {
            title: "FORMAT",
            args: vec![Arg {
                text: "text",
                optional: false,
                tooltip: Some(include_str!("./format_tooltip.txt").to_string()),
            }],
            subcommands: None,
        },
        Command {
            title: "CTCP",
            args: vec![
                Arg {
                    text: "nick",
                    optional: false,
                    tooltip: None,
                },
                Arg {
                    text: "command",
                    optional: false,
                    tooltip: Some(
                        "ACTION CLIENTINFO PING SOURCE VERSION TIME"
                            .to_string(),
                    ),
                },
                Arg {
                    text: "params",
                    optional: true,
                    tooltip: Some("Additional parameters".to_string()),
                },
            ],
            subcommands: None,
        },
    ]
});

fn isupport_parameter_to_command(
    isupport_parameter: &isupport::Parameter,
) -> Option<Command> {
    match isupport_parameter {
        isupport::Parameter::KNOCK => Some(KNOCK_COMMAND.clone()),
        isupport::Parameter::USERIP => Some(USERIP_COMMAND.clone()),
        isupport::Parameter::CNOTICE => Some(CNOTICE_COMMAND.clone()),
        isupport::Parameter::CPRIVMSG => Some(CPRIVMSG_COMMAND.clone()),
        _ => None,
    }
}

fn away_command(max_len: &u16) -> Command {
    Command {
        title: "AWAY",
        args: vec![Arg {
            text: "reason",
            optional: true,
            tooltip: Some(format!("maximum length: {max_len}")),
        }],
        subcommands: None,
    }
}

fn chathistory_command(maximum_limit: &u16) -> Command {
    Command {
        title: "CHATHISTORY",
        args: vec![Arg {
            text: "subcommand",
            optional: false,
            tooltip: Some(String::from(
                " BEFORE: Request messages before a timestamp or msgid\
               \n  AFTER: Request after before a timestamp or msgid\
               \n LATEST: Request most recent messages that have been sent\
               \n AROUND: Request messages before or after a timestamp or msgid\
               \nBETWEEN: Request messages between a timestamp or msgid and another timestamp or msgid\
               \nTARGETS: List channels with visible history and users that have sent direct messages",
            )),
        }],
        subcommands: Some(vec![
            chathistory_after_command(maximum_limit),
            chathistory_around_command(maximum_limit),
            chathistory_before_command(maximum_limit),
            chathistory_between_command(maximum_limit),
            chathistory_latest_command(maximum_limit),
            chathistory_targets_command(maximum_limit),
        ]),
    }
}

fn chathistory_after_command(maximum_limit: &u16) -> Command {
    let limit_tooltip = if *maximum_limit == 1 {
        String::from("up to 1 message")
    } else {
        format!("up to {maximum_limit} messages")
    };

    Command {
        title: "CHATHISTORY AFTER",
        args: vec![
            Arg {
                text: "target",
                optional: false,
                tooltip: None,
            },
            Arg {
                text: "timestamp | msgid",
                optional: false,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Arg {
                text: "limit",
                optional: false,
                tooltip: Some(limit_tooltip),
            },
        ],
        subcommands: None,
    }
}

fn chathistory_around_command(maximum_limit: &u16) -> Command {
    let limit_tooltip = if *maximum_limit == 1 {
        String::from("up to 1 message")
    } else {
        format!("up to {maximum_limit} messages")
    };

    Command {
        title: "CHATHISTORY AROUND",
        args: vec![
            Arg {
                text: "target",
                optional: false,
                tooltip: None,
            },
            Arg {
                text: "timestamp | msgid",
                optional: false,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Arg {
                text: "limit",
                optional: false,
                tooltip: Some(limit_tooltip),
            },
        ],
        subcommands: None,
    }
}

fn chathistory_before_command(maximum_limit: &u16) -> Command {
    let limit_tooltip = if *maximum_limit == 1 {
        String::from("up to 1 message")
    } else {
        format!("up to {maximum_limit} messages")
    };

    Command {
        title: "CHATHISTORY BEFORE",
        args: vec![
            Arg {
                text: "target",
                optional: false,
                tooltip: None,
            },
            Arg {
                text: "timestamp | msgid",
                optional: false,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Arg {
                text: "limit",
                optional: false,
                tooltip: Some(limit_tooltip),
            },
        ],
        subcommands: None,
    }
}

fn chathistory_between_command(maximum_limit: &u16) -> Command {
    let limit_tooltip = if *maximum_limit == 1 {
        String::from("up to 1 message")
    } else {
        format!("up to {maximum_limit} messages")
    };

    Command {
        title: "CHATHISTORY BETWEEN",
        args: vec![
            Arg {
                text: "target",
                optional: false,
                tooltip: None,
            },
            Arg {
                text: "timestamp | msgid",
                optional: false,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Arg {
                text: "timestamp | msgid",
                optional: false,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Arg {
                text: "limit",
                optional: false,
                tooltip: Some(limit_tooltip),
            },
        ],
        subcommands: None,
    }
}

fn chathistory_latest_command(maximum_limit: &u16) -> Command {
    let limit_tooltip = if *maximum_limit == 1 {
        String::from("up to 1 message")
    } else {
        format!("up to {maximum_limit} messages")
    };

    Command {
        title: "CHATHISTORY LATEST",
        args: vec![
            Arg {
                text: "target",
                optional: false,
                tooltip: None,
            },
            Arg {
                text: "* | timestamp | msgid",
                optional: false,
                tooltip: Some(String::from(
                    "               *: no restriction on returned messages\
                   \ntimestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Arg {
                text: "limit",
                optional: false,
                tooltip: Some(limit_tooltip),
            },
        ],
        subcommands: None,
    }
}

fn chathistory_targets_command(maximum_limit: &u16) -> Command {
    let limit_tooltip = if *maximum_limit == 1 {
        String::from("up to 1 target")
    } else {
        format!("up to {maximum_limit} targets")
    };

    Command {
        title: "CHATHISTORY TARGETS",
        args: vec![
            Arg {
                text: "timestamp",
                optional: false,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Arg {
                text: "timestamp",
                optional: false,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Arg {
                text: "limit",
                optional: false,
                tooltip: Some(limit_tooltip),
            },
        ],
        subcommands: None,
    }
}

static CNOTICE_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
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
    subcommands: None,
});

static CPRIVMSG_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
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
    subcommands: None,
});

fn join_command(
    channel_len: Option<&u16>,
    channel_limits: Option<&Vec<isupport::ChannelLimit>>,
    key_len: Option<&u16>,
) -> Command {
    let mut channels_tooltip = String::from("comma-separated");

    if let Some(channel_len) = channel_len {
        channels_tooltip.push_str(
            format!("\nmaximum length of each: {channel_len}").as_str(),
        );
    }

    if let Some(channel_limits) = channel_limits {
        channel_limits.iter().for_each(|channel_limit| {
            if let Some(limit) = channel_limit.limit {
                channels_tooltip.push_str(
                    format!(
                        "\nup to {limit} {} channels per client",
                        channel_limit.prefix
                    )
                    .as_str(),
                );
            } else {
                channels_tooltip.push_str(
                    format!(
                        "\nunlimited {} channels per client",
                        channel_limit.prefix
                    )
                    .as_str(),
                );
            }
        });
    }

    let mut keys_tooltip = String::from("comma-separated");

    if let Some(key_len) = key_len {
        keys_tooltip
            .push_str(format!("\nmaximum length of each: {key_len}").as_str());
    }

    Command {
        title: "JOIN",
        args: vec![
            Arg {
                text: "channels",
                optional: false,
                tooltip: Some(channels_tooltip),
            },
            Arg {
                text: "keys",
                optional: true,
                tooltip: Some(keys_tooltip),
            },
        ],
        subcommands: None,
    }
}

fn kick_command(target_limit: Option<u16>, max_len: Option<&u16>) -> Command {
    let mut users_tooltip = String::from("comma-separated");

    if let Some(target_limit) = target_limit {
        users_tooltip.push_str(format!("\nup to {target_limit} user").as_str());
        if target_limit != 1 {
            users_tooltip.push('s');
        }
    }

    let comment_tooltip =
        max_len.map(|max_len| format!("maximum length: {max_len}"));

    Command {
        title: "KICK",
        args: vec![
            Arg {
                text: "channel",
                optional: false,
                tooltip: None,
            },
            Arg {
                text: "users",
                optional: false,
                tooltip: Some(users_tooltip),
            },
            Arg {
                text: "comment",
                optional: true,
                tooltip: comment_tooltip,
            },
        ],
        subcommands: None,
    }
}

static KNOCK_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
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
    subcommands: None,
});

static LIST_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "LIST",
    args: vec![Arg {
        text: "channels",
        optional: true,
        tooltip: Some(String::from("comma-separated")),
    }],
    subcommands: None,
});

fn list_command(
    search_extensions: Option<&String>,
    target_limit: Option<u16>,
) -> Command {
    let mut channels_tooltip = String::from("comma-separated");

    if let Some(target_limit) = target_limit {
        channels_tooltip
            .push_str(format!("\nup to {target_limit} channel").as_str());
        if target_limit != 1 {
            channels_tooltip.push('s');
        }
    }

    if let Some(search_extensions) = search_extensions {
        let elistconds_tooltip = search_extensions.chars().fold(
            String::from("comma-separated"),
            |tooltip, search_extension| {
                tooltip + match search_extension {
                    'C' => "\n  C<{#}: created < # min ago\n  C>{#}: created > # min ago",
                    'M' => "\n {mask}: matches mask",
                    'N' => "\n!{mask}: does not match mask",
                    'T' => {
                        "\n  T<{#}: topic changed < # min ago\n  T>{#}: topic changed > # min ago"
                    }
                    'U' => "\n   <{#}: fewer than # users\n   >{#}: more than # users",
                    _ => "",
                }
            },
        );

        Command {
            title: "LIST",
            args: vec![
                Arg {
                    text: "channels",
                    optional: true,
                    tooltip: Some(channels_tooltip),
                },
                Arg {
                    text: "elistconds",
                    optional: true,
                    tooltip: Some(elistconds_tooltip),
                },
            ],
            subcommands: None,
        }
    } else {
        Command {
            title: "LIST",
            args: vec![Arg {
                text: "channels",
                optional: true,
                tooltip: Some(channels_tooltip),
            }],
            subcommands: None,
        }
    }
}

fn monitor_command(target_limit: &Option<u16>) -> Command {
    Command {
        title: "MONITOR",
        args: vec![Arg {
            text: "subcommand",
            optional: false,
            tooltip: Some(String::from(
                "+: Add user(s) to list being monitored\n\
                 -: Remove user(s) from list being monitored\n\
                 C: Clear the list of users being monitored\n\
                 L: Get list of users being monitored\n\
                 S: For each user in the list being monitored, get their current status",
            )),
        }],
        subcommands: Some(vec![
            monitor_add_command(target_limit),
            MONITOR_REMOVE_COMMAND.clone(),
            MONITOR_CLEAR_COMMAND.clone(),
            MONITOR_LIST_COMMAND.clone(),
            MONITOR_STATUS_COMMAND.clone(),
        ]),
    }
}

fn monitor_add_command(target_limit: &Option<u16>) -> Command {
    let mut targets_tooltip = String::from("comma-separated users");

    if let Some(target_limit) = target_limit {
        targets_tooltip
            .push_str(format!("\nup to {target_limit} target").as_str());
        if *target_limit != 1 {
            targets_tooltip.push('s');
        }
        targets_tooltip.push_str(" in total");
    }

    Command {
        title: "MONITOR +",
        args: vec![Arg {
            text: "targets",
            optional: false,
            tooltip: Some(targets_tooltip),
        }],
        subcommands: None,
    }
}

static MONITOR_REMOVE_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "MONITOR -",
    args: vec![Arg {
        text: "targets",
        optional: false,
        tooltip: Some(String::from("comma-separated")),
    }],
    subcommands: None,
});

static MONITOR_CLEAR_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "MONITOR C",
    args: vec![],
    subcommands: None,
});

static MONITOR_LIST_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "MONITOR L",
    args: vec![],
    subcommands: None,
});

static MONITOR_STATUS_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "MONITOR S",
    args: vec![],
    subcommands: None,
});

fn msg_command(
    channel_membership_prefixes: Vec<char>,
    target_limit: Option<u16>,
) -> Command {
    let mut targets_tooltip = String::from(
        "comma-separated\n    {user}: user directly\n {channel}: all users in channel",
    );

    for channel_membership_prefix in channel_membership_prefixes {
        match channel_membership_prefix {
            '~' => targets_tooltip
                .push_str("\n~{channel}: all founders in channel"),
            '&' => targets_tooltip
                .push_str("\n&{channel}: all protected users in channel"),
            '!' => targets_tooltip
                .push_str("\n!{channel}: all protected users in channel"),
            '@' => targets_tooltip
                .push_str("\n@{channel}: all operators in channel"),
            '%' => targets_tooltip
                .push_str("\n%{channel}: all half-operators in channel"),
            '+' => targets_tooltip
                .push_str("\n+{channel}: all voiced users in channel"),
            _ => (),
        }
    }

    if let Some(target_limit) = target_limit {
        targets_tooltip
            .push_str(format!("\nup to {target_limit} target").as_str());
        if target_limit != 1 {
            targets_tooltip.push('s');
        }
    }

    Command {
        title: "MSG",
        args: vec![
            Arg {
                text: "targets",
                optional: false,
                tooltip: Some(targets_tooltip),
            },
            Arg {
                text: "text",
                optional: true,
                tooltip: None,
            },
        ],
        subcommands: None,
    }
}

fn names_command(target_limit: u16) -> Command {
    let mut channels_tooltip = String::from("comma-separated");

    channels_tooltip
        .push_str(format!("\nup to {target_limit} channel").as_str());
    if target_limit != 1 {
        channels_tooltip.push('s');
    }

    Command {
        title: "NAMES",
        args: vec![Arg {
            text: "channels",
            optional: false,
            tooltip: Some(channels_tooltip),
        }],
        subcommands: None,
    }
}

fn nick_command(max_len: &u16) -> Command {
    Command {
        title: "NICK",
        args: vec![Arg {
            text: "nickname",
            optional: false,
            tooltip: Some(format!("maximum length: {max_len}")),
        }],
        subcommands: None,
    }
}

fn notice_command(
    channel_membership_prefixes: Vec<char>,
    target_limit: Option<u16>,
) -> Command {
    let mut targets_tooltip = String::from(
        "comma-separated\n    {user}: user directly\n {channel}: all users in channel",
    );

    for channel_membership_prefix in channel_membership_prefixes {
        match channel_membership_prefix {
            '~' => targets_tooltip
                .push_str("\n~{channel}: all founders in channel"),
            '&' => targets_tooltip
                .push_str("\n&{channel}: all protected users in channel"),
            '!' => targets_tooltip
                .push_str("\n!{channel}: all protected users in channel"),
            '@' => targets_tooltip
                .push_str("\n@{channel}: all operators in channel"),
            '%' => targets_tooltip
                .push_str("\n%{channel}: all half-operators in channel"),
            '+' => targets_tooltip
                .push_str("\n+{channel}: all voiced users in channel"),
            _ => (),
        }
    }

    if let Some(target_limit) = target_limit {
        targets_tooltip
            .push_str(format!("\nup to {target_limit} target").as_str());
        if target_limit != 1 {
            targets_tooltip.push('s');
        }
    }

    Command {
        title: "NOTICE",
        args: vec![
            Arg {
                text: "targets",
                optional: false,
                tooltip: Some(targets_tooltip),
            },
            Arg {
                text: "text",
                optional: true,
                tooltip: None,
            },
        ],
        subcommands: None,
    }
}

fn part_command(max_len: &u16) -> Command {
    Command {
        title: "PART",
        args: vec![
            Arg {
                text: "channels",
                optional: false,
                tooltip: Some(format!(
                    "comma-separated\nmaximum length of each: {max_len}"
                )),
            },
            Arg {
                text: "reason",
                optional: true,
                tooltip: None,
            },
        ],
        subcommands: None,
    }
}

fn setname_command(max_len: &u16) -> Command {
    Command {
        title: "SETNAME",
        args: vec![Arg {
            text: "realname",
            optional: false,
            tooltip: Some(format!("maximum length: {max_len}")),
        }],
        subcommands: None,
    }
}

fn topic_command(max_len: &u16) -> Command {
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
                tooltip: Some(format!("maximum length: {max_len}")),
            },
        ],
        subcommands: None,
    }
}

static USERIP_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "USERIP",
    args: vec![Arg {
        text: "nickname",
        optional: false,
        tooltip: None,
    }],
    subcommands: None,
});

static WHOX_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
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
    subcommands: None,
});

fn whois_command(target_limit: u16) -> Command {
    let mut nicks_tooltip = String::from("comma-separated");

    nicks_tooltip.push_str(format!("\nup to {target_limit} nick").as_str());
    if target_limit != 1 {
        nicks_tooltip.push('s');
    }

    Command {
        title: "WHOIS",
        args: vec![Arg {
            text: "nicks",
            optional: false,
            tooltip: Some(nicks_tooltip),
        }],
        subcommands: None,
    }
}

#[derive(Debug, Clone)]
enum Emojis {
    Idle,
    Selecting {
        highlighted: Option<usize>,
        filtered: Vec<&'static str>,
    },
    Selected {
        emoji: &'static str,
    },
}

impl Default for Emojis {
    fn default() -> Self {
        Self::Idle
    }
}

impl Emojis {
    fn process(&mut self, last_word: &str, config: &Config) {
        let last_word = last_word.strip_prefix(":").unwrap_or("");

        if last_word.is_empty() {
            *self = Self::default();
            return;
        }

        if let Some(shortcode) = config
            .buffer
            .emojis
            .auto_replace
            .then(|| last_word.strip_suffix(":"))
            .flatten()
            .map(|last_word| last_word.to_lowercase())
        {
            if let Some(emoji) =
                pick_emoji(&shortcode, config.buffer.emojis.skin_tone)
            {
                *self = Emojis::Selected { emoji };

                return;
            }
        } else if !config.buffer.emojis.show_picker {
            *self = Self::default();
            return;
        }

        let last_word = last_word
            .strip_suffix(":")
            .unwrap_or(last_word)
            .to_lowercase();

        let mut filtered = emojis::iter()
            .flat_map(|emoji| {
                emoji.shortcodes().filter_map(|shortcode| {
                    if shortcode.contains(&last_word) {
                        Some(FilteredShortcode {
                            similarity: jaro_winkler(&last_word, shortcode),
                            shortcode,
                        })
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<_>>();

        filtered.sort_by(|a, b| b.similarity.total_cmp(&a.similarity));

        *self = Emojis::Selecting {
            highlighted: Some(0),
            filtered: filtered.into_iter().map(|f| f.shortcode).collect(),
        };
    }

    fn select(&mut self, config: &Config) -> Option<String> {
        if let Self::Selecting {
            highlighted: Some(index),
            filtered,
        } = self
        {
            if let Some(shortcode) = filtered.get(*index).copied() {
                *self = Self::Idle;

                return pick_emoji(shortcode, config.buffer.emojis.skin_tone)
                    .map(|emoji| emoji.to_string());
            }
        }

        None
    }

    fn tab(&mut self, reverse: bool) -> bool {
        if let Self::Selecting {
            highlighted,
            filtered,
        } = self
        {
            selecting_tab(highlighted, filtered, reverse);

            true
        } else {
            false
        }
    }

    fn view<'a, Message: 'a>(
        &self,
        config: &Config,
    ) -> Option<Element<'a, Message>> {
        match self {
            Self::Idle | Self::Selected { .. } => None,
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

                    let to = index.max(MAX_SHOWN_EMOJI_ENTRIES - 1);
                    to.saturating_sub(MAX_SHOWN_EMOJI_ENTRIES - 1)
                };

                let entries = filtered
                    .iter()
                    .enumerate()
                    .skip(skip)
                    .take(MAX_SHOWN_EMOJI_ENTRIES)
                    .collect::<Vec<_>>();

                let content = |width| {
                    column(entries.iter().map(|(index, shortcode)| {
                        let selected = Some(*index) == *highlighted;
                        let content = text(format!(
                            "{} :{}:",
                            pick_emoji(
                                shortcode,
                                config.buffer.emojis.skin_tone
                            )
                            .unwrap_or(" "),
                            shortcode
                        ))
                        .shaping(text::Shaping::Advanced);

                        Element::from(
                            container(content)
                                .width(width)
                                .style(if selected {
                                    theme::container::primary_background_hover
                                } else {
                                    theme::container::none
                                })
                                .padding(6)
                                .center_y(Length::Shrink),
                        )
                    }))
                };

                (!entries.is_empty()).then(|| {
                    let first_pass = content(Length::Shrink);
                    let second_pass = content(Length::Fill);

                    container(double_pass(first_pass, second_pass))
                        .padding(4)
                        .style(theme::container::tooltip)
                        .width(Length::Shrink)
                        .into()
                })
            }
        }
    }
}

struct FilteredShortcode {
    similarity: f64,
    shortcode: &'static str,
}

fn pick_emoji(shortcode: &str, skin_tone: SkinTone) -> Option<&'static str> {
    emojis::get_by_shortcode(shortcode).map(|emoji| {
        if let Some(emoji_with_skin_tone) =
            emoji.with_skin_tone(skin_tone.into())
        {
            emoji_with_skin_tone
        } else {
            emoji
        }
        .as_str()
    })
}

fn replace_last_word_with_emoji(input: &str, emoji: &str) -> String {
    let mut words: Vec<_> = input.split(' ').collect();

    if let Some(last_word) = words.last_mut() {
        *last_word = emoji;
    }

    words.join(" ")
}

fn selecting_tab<T>(
    highlighted: &mut Option<usize>,
    filtered: &[T],
    reverse: bool,
) {
    if filtered.is_empty() {
        *highlighted = None;
    } else if let Some(index) = highlighted {
        if reverse {
            if *index > 0 {
                *index -= 1;
            } else {
                *index = filtered.len() - 1;
            }
        } else {
            *index = (*index + 1) % filtered.len();
        }
    } else {
        *highlighted = Some(if reverse { filtered.len() - 1 } else { 0 });
    }
}

pub enum Arrow {
    Up,
    Down,
}
