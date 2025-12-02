use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::LazyLock;
use std::{fmt, iter};

use chrono::{DateTime, Utc};
use const_format::concatcp;
use data::buffer::SkinTone;
use data::config::buffer::text_input::{OrderBy, SortDirection};
use data::history::filter::FilterChain;
use data::isupport::{self, find_target_limit};
use data::target::{self, Target};
use data::user::{ChannelUsers, Nick, NickRef};
use data::{Config, mode};
use iced::Length;
use iced::widget::{column, container, row, text, text_editor, tooltip};
use irc::proto;
use itertools::{Either, Itertools};
use strsim::jaro_winkler;

use crate::font;
use crate::theme::{self, Theme};
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
        cursor_position: usize,
        our_nickname: Option<NickRef>,
        users: Option<&ChannelUsers>,
        filters: FilterChain,
        last_seen: &HashMap<Nick, DateTime<Utc>>,
        channels: &[target::Channel],
        current_target: Option<&Target>,
        supports_detach: bool,
        isupport: &HashMap<isupport::Kind, isupport::Parameter>,
        config: &Config,
    ) {
        let is_command = input.starts_with('/');

        if is_command {
            self.commands.process(
                input,
                our_nickname,
                current_target,
                supports_detach,
                isupport,
            );

            // Disallow other completions when selecting a command
            if matches!(self.commands, Commands::Selecting { .. }) {
                self.text = Text::default();
                self.emojis = Emojis::default();

                return;
            }
        } else {
            self.commands = Commands::default();
        }

        if let Some(shortcode) = (config.buffer.emojis.show_picker
            || config.buffer.emojis.auto_replace)
            .then(|| {
                get_word(input, cursor_position)
                    .filter(|word| word.starts_with(':'))
            })
            .flatten()
        {
            self.emojis.process(shortcode, config);

            self.text = Text::default();
        } else {
            let casemapping =
                if let Some(isupport::Parameter::CASEMAPPING(casemapping)) =
                    isupport.get(&isupport::Kind::CASEMAPPING)
                {
                    *casemapping
                } else {
                    isupport::CaseMap::default()
                };

            self.text.process(
                input,
                cursor_position,
                casemapping,
                users,
                filters,
                last_seen,
                channels,
                current_target,
                config,
            );

            self.emojis = Emojis::default();
        }
    }

    pub fn select(&mut self, config: &Config) -> Option<Entry> {
        self.commands
            .select()
            .map(Entry::Command)
            .or(self.emojis.select(config).map(Entry::Emoji))
    }

    pub fn complete_emoji(
        &self,
        input: &str,
        cursor_position: usize,
    ) -> Option<Vec<text_editor::Action>> {
        if let Emojis::Selected { emoji } = self.emojis {
            Some(replace_word_with_text(input, cursor_position, emoji, None))
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
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        let command_view = self.commands.view(input, config, theme);
        let emojis_view = self.emojis.view(config);

        if command_view.is_some() || emojis_view.is_some() {
            Some(column![emojis_view, command_view].spacing(4).into())
        } else {
            None
        }
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
        cursor_position: usize,
        chantypes: &[char],
        config: &Config,
    ) -> Vec<text_editor::Action> {
        match self {
            Entry::Command(command) => replace_word_with_text(
                input,
                cursor_position,
                &format!("/{}", command.title.to_lowercase()),
                None,
            ),
            Entry::Text {
                next,
                append_suffix,
            } => {
                let autocomplete = &config.buffer.text_input.autocomplete;
                let is_channel = next.starts_with(chantypes);

                // If next is not the original prompt, then append the
                // configured suffix
                let suffix = if *append_suffix {
                    if input.trim_end().find(' ').is_none_or(|space_position| {
                        cursor_position <= space_position
                    }) && !is_channel
                    {
                        // If completed at the beginning of the input line and
                        // not a channel.
                        Some(autocomplete.completion_suffixes[0].as_str())
                    } else {
                        // Otherwise, use second suffix.
                        Some(autocomplete.completion_suffixes[1].as_str())
                    }
                } else {
                    None
                };

                replace_word_with_text(input, cursor_position, next, suffix)
            }
            Entry::Emoji(emoji) => {
                replace_word_with_text(input, cursor_position, emoji, None)
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
enum Commands {
    #[default]
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

impl Commands {
    fn process(
        &mut self,
        input: &str,
        our_nickname: Option<NickRef>,
        current_target: Option<&Target>,
        supports_detach: bool,
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

        let mut command_list = vec![
            // MOTD
            {
                Command {
                    title: "MOTD",
                    args: vec![Argument {
                        text: "server",
                        kind: ArgumentKind::Optional { skipped: false },
                        tooltip: None,
                    }],
                    subcommands: None,
                }
            },
            // QUIT
            {
                Command {
                    title: "QUIT",
                    args: vec![Argument {
                        text: "reason",
                        kind: ArgumentKind::Optional { skipped: false },
                        tooltip: None,
                    }],
                    subcommands: None,
                }
            },
            // Away
            {
                let max_len = match isupport.get(&isupport::Kind::AWAYLEN) {
                    Some(isupport::Parameter::AWAYLEN(len)) => Some(*len),
                    _ => None,
                };

                away_command(max_len)
            },
            // JOIN
            {
                {
                    let channel_len =
                        match isupport.get(&isupport::Kind::CHANNELLEN) {
                            Some(isupport::Parameter::CHANNELLEN(len)) => {
                                Some(*len)
                            }
                            _ => None,
                        };

                    let channel_limits = match isupport
                        .get(&isupport::Kind::CHANLIMIT)
                    {
                        Some(isupport::Parameter::CHANLIMIT(len)) => Some(len),
                        _ => None,
                    };

                    let key_len = match isupport.get(&isupport::Kind::KEYLEN) {
                        Some(isupport::Parameter::KEYLEN(len)) => Some(*len),
                        _ => None,
                    };

                    join_command(channel_len, channel_limits, key_len)
                }
            },
            // KICK
            {
                let default = current_target
                    .and_then(|target| target.as_channel())
                    .map(target::Channel::to_string);

                let kick_len = match isupport.get(&isupport::Kind::KICKLEN) {
                    Some(isupport::Parameter::KICKLEN(len)) => Some(*len),
                    _ => None,
                };

                let target_limit = find_target_limit(isupport, "KICK");

                kick_command(default, target_limit, kick_len)
            },
            // MSG
            {
                let channel_membership_prefixes: &[char] =
                    match isupport.get(&isupport::Kind::STATUSMSG) {
                        Some(isupport::Parameter::STATUSMSG(len)) => len,
                        _ => &[],
                    };

                let target_limit = find_target_limit(isupport, "PRIVMSG");

                msg_command(channel_membership_prefixes, target_limit)
            },
            // NAMES
            {
                let target_limit = find_target_limit(isupport, "NAMES");

                names_command(target_limit)
            },
            // NICK
            {
                let nick_len = match isupport.get(&isupport::Kind::NICKLEN) {
                    Some(isupport::Parameter::NICKLEN(len)) => Some(*len),
                    _ => None,
                };

                nick_command(nick_len)
            },
            // NOTICE
            {
                let channel_membership_prefixes: &[char] =
                    match isupport.get(&isupport::Kind::STATUSMSG) {
                        Some(isupport::Parameter::STATUSMSG(len)) => len,
                        _ => &[],
                    };

                let target_limit = find_target_limit(isupport, "NOTICE");

                notice_command(channel_membership_prefixes, target_limit)
            },
            // PART
            {
                let default = current_target.map(Target::to_string);

                let channel_len = match isupport
                    .get(&isupport::Kind::CHANNELLEN)
                {
                    Some(isupport::Parameter::CHANNELLEN(len)) => Some(*len),
                    _ => None,
                };

                part_command(default, channel_len)
            },
            // TOPIC
            {
                let default = current_target
                    .and_then(|target| target.as_channel())
                    .map(target::Channel::to_string);

                let max_len = match isupport.get(&isupport::Kind::TOPICLEN) {
                    Some(isupport::Parameter::TOPICLEN(len)) => Some(*len),
                    _ => None,
                };

                topic_command(default, max_len)
            },
            // WHO -- WHOX
            {
                if isupport.get(&isupport::Kind::WHOX).is_some() {
                    whox_command()
                } else {
                    who_command()
                }
            },
            // WHOIS
            {
                let target_limit = find_target_limit(isupport, "WHOIS");
                whois_command(target_limit)
            },
            // WHOWAS
            {
                Command {
                    title: "WHOWAS",
                    args: vec![
                        Argument {
                            text: "nick",
                            kind: ArgumentKind::Required,
                            tooltip: None,
                        },
                        Argument {
                            text: "count",
                            kind: ArgumentKind::Optional { skipped: false },
                            tooltip: Some(String::from(
                                "maximum number of nickname history entries returned, or all if omitted",
                            )),
                        },
                    ],
                    subcommands: None,
                }
            },
            // ME
            {
                Command {
                    title: "ME",
                    args: vec![Argument {
                        text: "action",
                        kind: ArgumentKind::Required,
                        tooltip: None,
                    }],
                    subcommands: None,
                }
            },
            // MODE
            {
                let chanmodes = isupport::get_chanmodes_or_default(isupport);
                let prefix = isupport::get_prefix_or_default(isupport);
                let mode_limit = isupport::get_mode_limit_or_default(isupport);

                let default = current_target
                    .map(Target::to_string)
                    .or(our_nickname.map(|nickname| nickname.to_string()));

                let mut tooltip = String::from("a channel or user");

                if let Some(ref default) = default {
                    tooltip.push_str(
                        format!("\nmay be skipped (default: {default})")
                            .as_str(),
                    );
                }

                Command {
                    title: "MODE",
                    args: vec![Argument {
                        text: "target",
                        kind: if default.is_some() {
                            ArgumentKind::Optional { skipped: false }
                        } else {
                            ArgumentKind::Required
                        },
                        tooltip: Some(tooltip),
                    }],
                    subcommands: Some(vec![
                        mode_channel_command(chanmodes, prefix, mode_limit),
                        mode_user_command(mode_limit),
                    ]),
                }
            },
            // RAW
            {
                Command {
                    title: "RAW",
                    args: vec![
                        Argument {
                            text: "command",
                            kind: ArgumentKind::Required,
                            tooltip: None,
                        },
                        Argument {
                            text: "args",
                            kind: ArgumentKind::Optional { skipped: false },
                            tooltip: None,
                        },
                    ],
                    subcommands: None,
                }
            },
            // FORMAT
            {
                Command {
                    title: "FORMAT",
                    args: vec![Argument {
                        text: "text",
                        kind: ArgumentKind::Required,
                        tooltip: Some(
                            include_str!("./format_tooltip.txt").to_string(),
                        ),
                    }],
                    subcommands: None,
                }
            },
            // PLAIN
            {
                Command {
                    title: "PLAIN",
                    args: vec![Argument {
                        text: "text",
                        kind: ArgumentKind::Required,
                        tooltip: None,
                    }],
                    subcommands: None,
                }
            },
            // HOP
            {
                Command {
                    title: "HOP",
                    args: vec![
                        Argument {
                            text: "channel",
                            kind: ArgumentKind::Optional { skipped: false },
                            tooltip: Some(String::from("the channel to join")),
                        },
                        Argument {
                            text: "message",
                            kind: ArgumentKind::Optional { skipped: false },
                            tooltip: Some(String::from(
                                "the part message to be sent",
                            )),
                        },
                    ],
                    subcommands: None,
                }
            },
            // SYSINFO
            {
                Command {
                    title: "SYSINFO",
                    args: vec![],
                    subcommands: None,
                }
            },
            // CLEAR
            {
                Command {
                    title: "CLEAR",
                    args: vec![],
                    subcommands: None,
                }
            },
            // CLEARTOPIC
            {
                let default = current_target
                    .and_then(|target| target.as_channel())
                    .map(target::Channel::to_string);

                Command {
                    title: "CLEARTOPIC",
                    args: vec![Argument {
                        text: "channel",
                        kind: if default.is_some() {
                            ArgumentKind::Optional { skipped: false }
                        } else {
                            ArgumentKind::Required
                        },
                        tooltip: default.map(|default| {
                            format!("may be omitted (default: {default})")
                        }),
                    }],
                    subcommands: None,
                }
            },
            // CTCP
            {
                let default = current_target
                    .and_then(|target| target.as_query())
                    .map(target::Query::to_string);

                Command {
                title: "CTCP",
                args: vec![
                    Argument {
                        text: "nick",
                        kind: if default.is_some() {
                            ArgumentKind::Optional { skipped: false }
                        } else {
                            ArgumentKind::Required
                        },
                        tooltip: default.map(|default| {
                            format!("may be skipped (default: {default})")
                        }),
                    },
                    Argument {
                        text: "command",
                        kind: ArgumentKind::Required,
                        tooltip: Some(
                            "    ACTION: Display <text> as a third-person action or emote\
                           \nCLIENTINFO: Request a list of the CTCP messages <nick> supports\
                           \n      PING: Request a reply containing the same <info> that was sent\
                           \n    SOURCE: Request a URL where the source code for <nick>'s IRC client can be found\
                           \n      TIME: Request the <nick>'s local time in a human-readable format\
                           \n  USERINFO: Request miscellaneous information about the user\
                           \n   VERSION: Request the name and version of <nick>'s IRC client".to_string(),
                        ),
                    },
                ],
                subcommands: Some(vec![
                        ctcp_action_command(),
                        ctcp_clientinfo_command(),
                        ctcp_userinfo_command(),
                        ctcp_ping_command(),
                        ctcp_source_command(),
                        ctcp_time_command(),
                        ctcp_version_command()
                    ]),
            }
            },
        ];

        if supports_detach {
            let default = current_target
                .and_then(|target| target.as_channel())
                .map(target::Channel::to_string);

            let channel_len = match isupport.get(&isupport::Kind::CHANNELLEN) {
                Some(isupport::Parameter::CHANNELLEN(len)) => Some(*len),
                _ => None,
            };

            command_list.push(detach_command(default, channel_len));
        }

        let isupport_commands = isupport
            .iter()
            .filter_map(|(_, isupport_parameter)| match isupport_parameter {
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

                    if search_extensions.is_some() || target_limit.is_some() {
                        Some(list_command(search_extensions, target_limit))
                    } else {
                        Some(LIST_COMMAND.clone())
                    }
                }
                isupport::Parameter::NAMELEN(max_len) => {
                    Some(setname_command(max_len))
                }
                _ => isupport_parameter_to_command(isupport_parameter),
            })
            .collect::<Vec<Command>>();

        command_list.extend(isupport_commands);

        match self {
            // Command not fully typed, show filtered entries
            _ if !has_space => {
                if let Some(command) = command_list.iter().find(|command| {
                    command.title.to_lowercase() == cmd.to_lowercase()
                }) {
                    *self = Self::Selected {
                        command: command.clone(),
                        subcommand: None,
                    };
                } else {
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
            // Command fully typed & already selected
            Self::Selected { .. } => {}
        }

        if let Self::Selected { command, .. } = self {
            // Mark skipped arguments as skipped
            match command.title {
                "CTCP" => {
                    if let Some(nick) = rest.split_ascii_whitespace().nth(1)
                        && matches!(
                            nick.to_uppercase().as_str(),
                            "ACTION"
                                | "CLIENTINFO"
                                | "PING"
                                | "SOURCE"
                                | "TIME"
                                | "VERSION"
                        )
                        && let Some(nick) = command.args.get_mut(0)
                    {
                        nick.kind.skip();
                    }
                }
                "KICK" => {
                    if let Some(channel) = rest.split_ascii_whitespace().nth(1)
                    {
                        let chantypes =
                            isupport::get_chantypes_or_default(isupport);

                        if !proto::is_channel(channel, chantypes)
                            && let Some(channel) = command.args.get_mut(0)
                        {
                            channel.kind.skip();
                        }
                    }
                }
                "MODE" => {
                    if let Some(target) = rest.split_ascii_whitespace().nth(1)
                        && target.starts_with(['+', '-'])
                        && let Some(target) = command.args.get_mut(0)
                    {
                        target.kind.skip();
                    }
                }
                "TOPIC" => {
                    if let Some(channel) = rest.split_ascii_whitespace().nth(1)
                    {
                        let chantypes =
                            isupport::get_chantypes_or_default(isupport);

                        if !proto::is_channel(channel, chantypes)
                            && let Some(channel) = command.args.get_mut(0)
                        {
                            channel.kind.skip();
                        }
                    }
                }
                _ => (),
            }

            // Check for subcommand, if any exist
            if let Some(subcommands) = &command.subcommands {
                if let Some(subcmd) =
                    rest[cmd.len()..].split_ascii_whitespace().nth(
                        command
                            .args
                            .iter()
                            .filter(|arg| !arg.kind.skipped())
                            .count()
                            .saturating_sub(1),
                    )
                {
                    let subcmd = (String::from(command.title) + " " + subcmd)
                        .to_lowercase();

                    let subcommand = subcommands.iter().find(|subcommand| {
                        subcommand.title.to_lowercase() == subcmd
                            || subcommand
                                .alias()
                                .iter()
                                .any(|alias| alias.to_lowercase() == subcmd)
                    });

                    *self = Self::Selected {
                        command: command.clone(),
                        subcommand: subcommand.cloned(),
                    };
                } else {
                    *self = Self::Selected {
                        command: command.clone(),
                        subcommand: None,
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
            && let Some(command) = filtered.get(*index).cloned()
        {
            *self = Self::Selected {
                command: command.clone(),
                subcommand: None,
            };

            return Some(command);
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
        theme: &'a Theme,
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
                    Some(command.view(input, subcommand.as_ref(), theme))
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
    args: Vec<Argument>,
    subcommands: Option<Vec<Command>>,
}

const MODE_CHANNEL_PATTERN: &str =
    concatcp!("mode ", REQUIRED_ARG_PREFIX, "channel", REQUIRED_ARG_SUFFIX);
const MODE_USER_PATTERN: &str =
    concatcp!("mode ", REQUIRED_ARG_PREFIX, "user", REQUIRED_ARG_SUFFIX);

impl Command {
    fn description(&self) -> Option<&'static str> {
        Some(match self.title.to_lowercase().as_str() {
            "away" => {
                "Mark yourself as away. If already away, the status is removed"
            }
            "join" => "Join channel(s) with optional key(s)",
            "me" => "Send an action message to the channel",
            "mode" => "Set or retrieve target's mode(s)",
            MODE_CHANNEL_PATTERN => "Set or retrieve channel's mode(s)",
            MODE_USER_PATTERN => "Set or retrieve user's mode(s)",
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
            "whowas" => "Retrieve information about no longer present user(s)",
            "format" => "Format text using markdown or $ sequences",
            "plain" => "Send text with automatic formatting disabled",
            "ctcp" => "Send Client-To-Client requests",
            "ctcp action" => "Display <text> as a third-person action or emote",
            "ctcp clientinfo" => {
                "Request a list of the CTCP messages <nick> supports"
            }
            "ctcp ping" => {
                "Request a reply containing the same <info> that was sent"
            }
            "ctcp source" => {
                "Request a URL where the source code for <nick>'s IRC client can be found"
            }
            "ctcp time" => {
                "Request the <nick>'s local time in a human-readable format"
            }
            "ctcp version" => {
                "Request the name and version of <nick>'s IRC client"
            }
            "hop" => "Parts the current channel and joins a new one",
            "clear" => "Clears the buffer",
            "cleartopic" => "Clear the topic of a channel",
            "sysinfo" => "Send system information",
            "detach" => {
                "Hide the channel, leaving the bouncer's connection to the channel active"
            }
            _ => return None,
        })
    }

    fn alias(&self) -> Vec<&str> {
        match self.title.to_lowercase().as_str() {
            "away" => vec![],
            "join" => vec!["j"],
            "me" => vec!["describe"],
            "mode" => vec!["m"],
            "msg" => vec!["query"],
            "nick" => vec![],
            "part" => vec!["leave"],
            "quit" => vec![""],
            "raw" => vec![],
            "topic" => vec!["t"],
            "whois" => vec![],
            "format" => vec!["f"],
            "plain" => vec!["p"],
            "hop" => vec!["rejoin"],
            "clear" => vec![],
            "sysinfo" => vec![],
            _ => vec![],
        }
    }

    fn view<'a, Message: 'a>(
        &self,
        input: &str,
        subcommand: Option<&Command>,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let command_prefix = format!("/{}", self.title.to_lowercase());

        let num_skipped =
            self.args.iter().filter(|arg| arg.kind.skipped()).count()
                + subcommand.map_or(0, |subcommand| {
                    subcommand
                        .args
                        .iter()
                        .filter(|arg| arg.kind.skipped())
                        .count()
                });

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
        .saturating_add(num_skipped)
        .saturating_sub(2)
        .min(
            (self.args.len()
                + subcommand.map_or(0, |subcommand| subcommand.args.len()))
            .saturating_sub(1),
        );

        let title = Some(Element::from(text(self.title)));

        let arg_text = |index: usize, arg: &Argument| {
            let content = text(format!("{arg}"))
                .style(move |theme| {
                    if index == active_arg {
                        theme::text::tertiary(theme)
                    } else {
                        theme::text::none(theme)
                    }
                })
                .font_maybe(
                    theme::font_style::tertiary(theme)
                        .filter(|_| index == active_arg)
                        .map(font::get),
                );

            if let Some(arg_tooltip) = &arg.tooltip {
                let tooltip_indicator = text("*")
                    .style(move |theme| {
                        if index == active_arg {
                            theme::text::tertiary(theme)
                        } else {
                            theme::text::none(theme)
                        }
                    })
                    .font_maybe(
                        theme::font_style::tertiary(theme)
                            .filter(|_| index == active_arg)
                            .map(font::get),
                    )
                    .size(8);

                Element::from(row![
                    text(" "),
                    tooltip(
                        row![content, tooltip_indicator]
                            .align_y(iced::Alignment::Start),
                        container(
                            text(arg_tooltip.clone())
                                .style(move |theme| {
                                    if index == active_arg {
                                        theme::text::tertiary(theme)
                                    } else {
                                        theme::text::secondary(theme)
                                    }
                                })
                                .font_maybe(if index == active_arg {
                                    theme::font_style::tertiary(theme)
                                        .map(font::get)
                                } else {
                                    theme::font_style::secondary(theme)
                                        .map(font::get)
                                })
                        )
                        .style(theme::container::tooltip)
                        .padding(8),
                        tooltip::Position::Top,
                    )
                    .delay(iced::time::Duration::ZERO)
                ])
            } else {
                Element::from(row![text(" "), content])
            }
        };

        let args = if let Some(subcommand) = subcommand {
            Either::Left(
                self.args
                    .iter()
                    .take(self.args.len() - 1)
                    .enumerate()
                    .map(|(index, arg)| arg_text(index, arg))
                    .chain(std::iter::once(Element::from(row![
                        text(
                            subcommand
                                .title
                                .strip_prefix(self.title)
                                .unwrap_or_default()
                        )
                        .style(move |theme| {
                            if 0 == active_arg {
                                theme::text::tertiary(theme)
                            } else {
                                theme::text::none(theme)
                            }
                        })
                    ])))
                    .chain(subcommand.args.iter().enumerate().map(
                        |(index, arg)| arg_text(self.args.len() + index, arg),
                    )),
            )
        } else {
            let args = self
                .args
                .iter()
                .enumerate()
                .map(|(index, arg)| arg_text(index, arg));

            Either::Right(if self.subcommands.is_some() {
                Either::Left(args.chain(iter::once(Element::from(row![
                    text(" ...").style(theme::text::none)
                ]))))
            } else {
                Either::Right(args)
            })
        };

        container(column![
            subcommand
                .map_or(self.description(), |subcommand| {
                    subcommand.description()
                })
                .map(|description| {
                    text(description).style(theme::text::secondary).font_maybe(
                        theme::font_style::secondary(theme).map(font::get),
                    )
                }),
            row(title.into_iter().chain(args)),
        ])
        .style(theme::container::tooltip)
        .padding(8)
        .center_y(Length::Shrink)
        .into()
    }
}

#[derive(Debug, Clone)]
struct Argument {
    text: &'static str,
    kind: ArgumentKind,
    tooltip: Option<String>,
}

// Whether the argument can be skipped or omitted, and if so whether it has been
// skipped
#[derive(Debug, Clone)]
enum ArgumentKind {
    Required,
    Optional { skipped: bool },
}

impl ArgumentKind {
    fn skip(&mut self) {
        match self {
            ArgumentKind::Required => (),
            ArgumentKind::Optional { skipped } => *skipped = true,
        }
    }

    fn skipped(&self) -> bool {
        match self {
            ArgumentKind::Required => false,
            ArgumentKind::Optional { skipped } => *skipped,
        }
    }
}

const REQUIRED_ARG_PREFIX: &str = "<";
const REQUIRED_ARG_SUFFIX: &str = ">";

impl fmt::Display for Argument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if matches!(self.kind, ArgumentKind::Optional { .. }) {
            write!(f, "[<{}>]", self.text)
        } else {
            write!(
                f,
                "{}{}{}",
                REQUIRED_ARG_PREFIX, self.text, REQUIRED_ARG_SUFFIX
            )
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
        cursor_position: usize,
        casemapping: isupport::CaseMap,
        users: Option<&ChannelUsers>,
        filters: FilterChain,
        last_seen: &HashMap<Nick, DateTime<Utc>>,
        channels: &[target::Channel],
        current_target: Option<&Target>,
        config: &Config,
    ) {
        if !self.process_channels(
            input,
            cursor_position,
            casemapping,
            channels,
            current_target.and_then(Target::as_channel),
            config,
        ) {
            self.process_users(
                input,
                cursor_position,
                casemapping,
                users,
                filters,
                current_target.and_then(Target::as_channel),
                last_seen,
                config,
            );
        }
    }

    fn process_users(
        &mut self,
        input: &str,
        cursor_position: usize,
        casemapping: isupport::CaseMap,
        users: Option<&ChannelUsers>,
        filters: FilterChain,
        current_channel: Option<&target::Channel>,
        last_seen: &HashMap<Nick, DateTime<Utc>>,
        config: &Config,
    ) {
        let autocomplete = &config.buffer.text_input.autocomplete;

        let Some(word) = get_word(input, cursor_position) else {
            *self = Self::default();
            return;
        };

        let nick = casemapping.normalize(word);

        self.selected = None;
        self.prompt = word.to_string();
        self.filtered = users
            .into_iter()
            .flatten()
            .filter(|user| !filters.filter_user(user, current_channel))
            .sorted_by(|a, b| {
                if matches!(autocomplete.order_by, OrderBy::Recent) {
                    if let Some(a_last_seen) =
                        last_seen.get(&a.nickname().to_owned())
                    {
                        if let Some(b_last_seen) =
                            last_seen.get(&b.nickname().to_owned())
                        {
                            b_last_seen.cmp(a_last_seen)
                        } else {
                            Ordering::Less
                        }
                    } else if last_seen.get(&b.nickname().to_owned()).is_some()
                    {
                        Ordering::Greater
                    } else {
                        match autocomplete.sort_direction {
                            SortDirection::Asc => {
                                a.nickname().cmp(&b.nickname())
                            }
                            SortDirection::Desc => {
                                b.nickname().cmp(&a.nickname())
                            }
                        }
                    }
                } else {
                    match autocomplete.sort_direction {
                        SortDirection::Asc => a.nickname().cmp(&b.nickname()),
                        SortDirection::Desc => b.nickname().cmp(&a.nickname()),
                    }
                }
            })
            .filter(|&user| user.as_normalized_str().starts_with(&nick))
            .map(|user| user.nickname().to_string())
            .collect();
    }

    fn process_channels(
        &mut self,
        input: &str,
        cursor_position: usize,
        casemapping: isupport::CaseMap,
        channels: &[target::Channel],
        current_channel: Option<&target::Channel>,
        config: &Config,
    ) -> bool {
        let autocomplete = &config.buffer.text_input.autocomplete;

        let Some((_, rest)) = get_word(input, cursor_position)
            .and_then(|word| word.split_once('#'))
        else {
            *self = Self::default();
            return false;
        };

        let input_channel = format!("#{}", casemapping.normalize(rest));

        self.selected = None;
        self.prompt = format!("#{rest}");
        self.filtered = channels
            .iter()
            .sorted_by(|a, b: &&target::Channel| {
                if let Some(current_channel) = current_channel {
                    let a_is_current_channel = a.as_normalized_str()
                        == current_channel.as_normalized_str();
                    let b_is_current_channel = b.as_normalized_str()
                        == current_channel.as_normalized_str();

                    match (a_is_current_channel, b_is_current_channel) {
                        (false, false) => (),
                        (true, false) => return std::cmp::Ordering::Less,
                        (false, true) => return std::cmp::Ordering::Greater,
                        (true, true) => return std::cmp::Ordering::Equal,
                    }
                }

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
            .map(ToString::to_string)
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

fn away_command(max_len: Option<u16>) -> Command {
    let tooltip = max_len.map(|max_len| format!("maximum length: {max_len}"));

    Command {
        title: "AWAY",
        args: vec![Argument {
            text: "reason",
            kind: ArgumentKind::Optional { skipped: false },
            tooltip,
        }],
        subcommands: None,
    }
}

fn ctcp_action_command() -> Command {
    Command {
        title: "CTCP ACTION",
        args: vec![Argument {
            text: "text",
            kind: ArgumentKind::Required,
            tooltip: Some(String::from(
                "message to display as a third-person action or emote",
            )),
        }],
        subcommands: None,
    }
}

fn ctcp_clientinfo_command() -> Command {
    Command {
        title: "CTCP CLIENTINFO",
        args: vec![],
        subcommands: None,
    }
}

fn ctcp_userinfo_command() -> Command {
    Command {
        title: "CTCP USERINFO",
        args: vec![],
        subcommands: None,
    }
}

fn ctcp_ping_command() -> Command {
    Command {
        title: "CTCP PING",
        args: vec![Argument {
            text: "info",
            kind: ArgumentKind::Required,
            tooltip: Some(String::from(
                "text that should be exactly reproduced in the reply PING",
            )),
        }],
        subcommands: None,
    }
}

fn ctcp_source_command() -> Command {
    Command {
        title: "CTCP SOURCE",
        args: vec![],
        subcommands: None,
    }
}

fn ctcp_time_command() -> Command {
    Command {
        title: "CTCP TIME",
        args: vec![],
        subcommands: None,
    }
}

fn ctcp_version_command() -> Command {
    Command {
        title: "CTCP VERSION",
        args: vec![],
        subcommands: None,
    }
}

fn chathistory_command(maximum_limit: &u16) -> Command {
    Command {
        title: "CHATHISTORY",
        args: vec![Argument {
            text: "subcommand",
            kind: ArgumentKind::Required,
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
            Argument {
                text: "target",
                kind: ArgumentKind::Required,
                tooltip: None,
            },
            Argument {
                text: "timestamp | msgid",
                kind: ArgumentKind::Required,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Argument {
                text: "limit",
                kind: ArgumentKind::Required,
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
            Argument {
                text: "target",
                kind: ArgumentKind::Required,
                tooltip: None,
            },
            Argument {
                text: "timestamp | msgid",
                kind: ArgumentKind::Required,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Argument {
                text: "limit",
                kind: ArgumentKind::Required,
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
            Argument {
                text: "target",
                kind: ArgumentKind::Required,
                tooltip: None,
            },
            Argument {
                text: "timestamp | msgid",
                kind: ArgumentKind::Required,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Argument {
                text: "limit",
                kind: ArgumentKind::Required,
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
            Argument {
                text: "target",
                kind: ArgumentKind::Required,
                tooltip: None,
            },
            Argument {
                text: "timestamp | msgid",
                kind: ArgumentKind::Required,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Argument {
                text: "timestamp | msgid",
                kind: ArgumentKind::Required,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Argument {
                text: "limit",
                kind: ArgumentKind::Required,
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
            Argument {
                text: "target",
                kind: ArgumentKind::Required,
                tooltip: None,
            },
            Argument {
                text: "* | timestamp | msgid",
                kind: ArgumentKind::Required,
                tooltip: Some(String::from(
                    "               *: no restriction on returned messages\
                   \ntimestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Argument {
                text: "limit",
                kind: ArgumentKind::Required,
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
            Argument {
                text: "timestamp",
                kind: ArgumentKind::Required,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Argument {
                text: "timestamp",
                kind: ArgumentKind::Required,
                tooltip: Some(String::from(
                    "timestamp format: timestamp=YYYY-MM-DDThh:mm:ss.sssZ",
                )),
            },
            Argument {
                text: "limit",
                kind: ArgumentKind::Required,
                tooltip: Some(limit_tooltip),
            },
        ],
        subcommands: None,
    }
}

static CNOTICE_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "CNOTICE",
    args: vec![
        Argument {
            text: "nickname",
            kind: ArgumentKind::Required,
            tooltip: None,
        },
        Argument {
            text: "channel",
            kind: ArgumentKind::Required,
            tooltip: None,
        },
        Argument {
            text: "message",
            kind: ArgumentKind::Required,
            tooltip: None,
        },
    ],
    subcommands: None,
});

static CPRIVMSG_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "CPRIVMSG",
    args: vec![
        Argument {
            text: "nickname",
            kind: ArgumentKind::Required,
            tooltip: None,
        },
        Argument {
            text: "channel",
            kind: ArgumentKind::Required,
            tooltip: None,
        },
        Argument {
            text: "message",
            kind: ArgumentKind::Required,
            tooltip: None,
        },
    ],
    subcommands: None,
});

fn detach_command(
    default: Option<String>,
    channel_len: Option<u16>,
) -> Command {
    let mut channels_tooltip = String::from("comma-separated");

    if let Some(channel_len) = channel_len {
        channels_tooltip.push_str(
            format!("\nmaximum length of each: {channel_len}").as_str(),
        );
    }

    if let Some(default) = &default {
        channels_tooltip.push_str(
            format!("\nmay be skipped (default: {default})").as_str(),
        );
    }

    Command {
        title: "DETACH",
        args: vec![Argument {
            text: "channels",
            kind: if default.is_some() {
                ArgumentKind::Optional { skipped: false }
            } else {
                ArgumentKind::Required
            },
            tooltip: Some(channels_tooltip),
        }],
        subcommands: None,
    }
}

fn join_command(
    channel_len: Option<u16>,
    channel_limits: Option<&Vec<isupport::ChannelLimit>>,
    key_len: Option<u16>,
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
            Argument {
                text: "channels",
                kind: ArgumentKind::Required,
                tooltip: Some(channels_tooltip),
            },
            Argument {
                text: "keys",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: Some(keys_tooltip),
            },
        ],
        subcommands: None,
    }
}

fn kick_command(
    default: Option<String>,
    target_limit: Option<u16>,
    max_len: Option<u16>,
) -> Command {
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
            Argument {
                text: "channel",
                kind: if default.is_some() {
                    ArgumentKind::Optional { skipped: false }
                } else {
                    ArgumentKind::Required
                },
                tooltip: default.map(|default| {
                    format!("may be skipped (default: {default})")
                }),
            },
            Argument {
                text: "users",
                kind: ArgumentKind::Required,
                tooltip: Some(users_tooltip),
            },
            Argument {
                text: "comment",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: comment_tooltip,
            },
        ],
        subcommands: None,
    }
}

static KNOCK_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "KNOCK",
    args: vec![
        Argument {
            text: "channel",
            kind: ArgumentKind::Required,
            tooltip: None,
        },
        Argument {
            text: "message",
            kind: ArgumentKind::Optional { skipped: false },
            tooltip: None,
        },
    ],
    subcommands: None,
});

static LIST_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "LIST",
    args: vec![Argument {
        text: "channels",
        kind: ArgumentKind::Optional { skipped: false },
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
                Argument {
                    text: "channels",
                    kind: ArgumentKind::Optional { skipped: false },
                    tooltip: Some(channels_tooltip),
                },
                Argument {
                    text: "elistconds",
                    kind: ArgumentKind::Optional { skipped: false },
                    tooltip: Some(elistconds_tooltip),
                },
            ],
            subcommands: None,
        }
    } else {
        Command {
            title: "LIST",
            args: vec![Argument {
                text: "channels",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: Some(channels_tooltip),
            }],
            subcommands: None,
        }
    }
}

fn monitor_command(target_limit: &Option<u16>) -> Command {
    Command {
        title: "MONITOR",
        args: vec![Argument {
            text: "subcommand",
            kind: ArgumentKind::Required,
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
        args: vec![Argument {
            text: "targets",
            kind: ArgumentKind::Required,
            tooltip: Some(targets_tooltip),
        }],
        subcommands: None,
    }
}

static MONITOR_REMOVE_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "MONITOR -",
    args: vec![Argument {
        text: "targets",
        kind: ArgumentKind::Required,
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

fn mode_channel_command(
    chanmodes: &[isupport::ModeKind],
    prefix: &[isupport::PrefixMap],
    mode_limit: Option<u16>,
) -> Command {
    let mut modestring_tooltip = String::new();

    let mut unknown_modes = String::new();

    for chanmode in chanmodes.iter() {
        if !chanmode.modes.is_empty() {
            if !modestring_tooltip.is_empty() {
                modestring_tooltip.push('\n');
            }

            modestring_tooltip +=
                &format!("Type {} Modes ({chanmode})", chanmode.kind);

            for mode in chanmode.modes.chars() {
                let channel_mode = mode::Channel::from(mode);

                match channel_mode {
                    mode::Channel::Unknown(_) => unknown_modes.push(mode),
                    _ => {
                        modestring_tooltip +=
                            &format!("\n  {mode}: {channel_mode}");
                    }
                }
            }

            if let Some(unknown_mode) = unknown_modes.chars().next() {
                let unknown_mode = mode::Channel::from(unknown_mode);

                modestring_tooltip +=
                    &format!("\n  {unknown_modes}: {unknown_mode}");
                if unknown_modes.len() > 1 {
                    modestring_tooltip.push('s');
                }
            }

            unknown_modes.clear();
        }
    }

    if !prefix.is_empty() {
        if !modestring_tooltip.is_empty() {
            modestring_tooltip.push('\n');
        }

        modestring_tooltip +=
            "Membership Modes (requires nickname as argument)";
    }

    for prefix_map in prefix.iter() {
        modestring_tooltip += &format!(
            "\n  {}: {} ({})",
            prefix_map.mode,
            mode::Channel::from(prefix_map.prefix),
            prefix_map.prefix
        );
    }

    if !modestring_tooltip.is_empty() {
        modestring_tooltip += "\nmode descriptions are standard and/or well-used meanings, and may be inaccurate\n";
    }

    if let Some(mode_limit) = mode_limit {
        modestring_tooltip
            .push_str(format!("up to {mode_limit} channel mode").as_str());
        if mode_limit != 1 {
            modestring_tooltip.push('s');
        }
    } else {
        modestring_tooltip.push_str("unlimited channel modes");
    }

    Command {
        title: concatcp!(
            "MODE ",
            REQUIRED_ARG_PREFIX,
            "channel",
            REQUIRED_ARG_SUFFIX
        ),
        args: vec![
            Argument {
                text: "modestring",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: Some(modestring_tooltip),
            },
            Argument {
                text: "arguments",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: None,
            },
        ],
        subcommands: None,
    }
}

fn mode_user_command(mode_limit: Option<u16>) -> Command {
    let mut modestring_tooltip = String::new();

    if let Some(mode_limit) = mode_limit {
        modestring_tooltip
            .push_str(format!("up to {mode_limit} user mode").as_str());
        if mode_limit != 1 {
            modestring_tooltip.push('s');
        }
    } else {
        modestring_tooltip.push_str("unlimited user modes");
    }

    Command {
        title: concatcp!(
            "MODE ",
            REQUIRED_ARG_PREFIX,
            "user",
            REQUIRED_ARG_SUFFIX
        ),
        args: vec![Argument {
            text: "modestring",
            kind: ArgumentKind::Optional { skipped: false },
            tooltip: Some(modestring_tooltip),
        }],
        subcommands: None,
    }
}

fn msg_command(
    channel_membership_prefixes: &[char],
    target_limit: Option<u16>,
) -> Command {
    let mut targets_tooltip = String::from(
        "comma-separated\n    {user}: user directly\n {channel}: all users in channel",
    );

    for channel_membership_prefix in channel_membership_prefixes {
        match *channel_membership_prefix {
            proto::FOUNDER_PREFIX => targets_tooltip
                .push_str("\n~{channel}: all founders in channel"),
            proto::PROTECTED_PREFIX_STD | proto::PROTECTED_PREFIX_ALT => targets_tooltip
                .push_str("\n{channel_membership_prefix}{channel}: all protected users in channel"),
            proto::OPERATOR_PREFIX => targets_tooltip
                .push_str("\n@{channel}: all operators in channel"),
            proto::HALF_OPERATOR_PREFIX => targets_tooltip
                .push_str("\n%{channel}: all half-operators in channel"),
            proto::VOICED_PREFIX => targets_tooltip
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
            Argument {
                text: "targets",
                kind: ArgumentKind::Required,
                tooltip: Some(targets_tooltip),
            },
            Argument {
                text: "text",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: None,
            },
        ],
        subcommands: None,
    }
}

fn names_command(target_limit: Option<u16>) -> Command {
    let mut channels_tooltip = String::from("comma-separated");

    if let Some(target_limit) = target_limit {
        channels_tooltip
            .push_str(format!("\nup to {target_limit} channel").as_str());

        if target_limit != 1 {
            channels_tooltip.push('s');
        }
    }

    Command {
        title: "NAMES",
        args: vec![Argument {
            text: "channels",
            kind: ArgumentKind::Required,
            tooltip: Some(channels_tooltip),
        }],
        subcommands: None,
    }
}

fn nick_command(max_len: Option<u16>) -> Command {
    let tooltip = max_len.map(|max_len| format!("maximum length: {max_len}"));

    Command {
        title: "NICK",
        args: vec![Argument {
            text: "nickname",
            kind: ArgumentKind::Required,
            tooltip,
        }],
        subcommands: None,
    }
}

fn notice_command(
    channel_membership_prefixes: &[char],
    target_limit: Option<u16>,
) -> Command {
    let mut targets_tooltip = String::from(
        "comma-separated\n    {user}: user directly\n {channel}: all users in channel",
    );

    for channel_membership_prefix in channel_membership_prefixes {
        match *channel_membership_prefix {
            proto::FOUNDER_PREFIX => targets_tooltip
                .push_str("\n~{channel}: all founders in channel"),
            proto::PROTECTED_PREFIX_STD | proto::PROTECTED_PREFIX_ALT => targets_tooltip
                .push_str("\n{channel_membership_prefix}{channel}: all protected users in channel"),
            proto::OPERATOR_PREFIX => targets_tooltip
                .push_str("\n@{channel}: all operators in channel"),
            proto::HALF_OPERATOR_PREFIX => targets_tooltip
                .push_str("\n%{channel}: all half-operators in channel"),
            proto::VOICED_PREFIX => targets_tooltip
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
            Argument {
                text: "targets",
                kind: ArgumentKind::Required,
                tooltip: Some(targets_tooltip),
            },
            Argument {
                text: "text",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: None,
            },
        ],
        subcommands: None,
    }
}

fn part_command(default: Option<String>, max_len: Option<u16>) -> Command {
    let mut targets_tooltip =
        String::from("channels and/or queries, comma-separated");

    if let Some(max_len) = max_len {
        targets_tooltip.push_str(
            format!("\nmaximum length of each channel: {max_len}").as_str(),
        );
    }

    if let Some(ref default) = default {
        targets_tooltip.push_str(
            format!("\nmay be omitted (default: {default})").as_str(),
        );
    }

    Command {
        title: "PART",
        args: vec![
            Argument {
                text: "targets",
                kind: if default.is_some() {
                    ArgumentKind::Optional { skipped: false }
                } else {
                    ArgumentKind::Required
                },
                tooltip: Some(targets_tooltip),
            },
            Argument {
                text: "reason",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: None,
            },
        ],
        subcommands: None,
    }
}

fn setname_command(max_len: &u16) -> Command {
    Command {
        title: "SETNAME",
        args: vec![Argument {
            text: "realname",
            kind: ArgumentKind::Required,
            tooltip: Some(format!("maximum length: {max_len}")),
        }],
        subcommands: None,
    }
}

fn topic_command(default: Option<String>, max_len: Option<u16>) -> Command {
    let mut topic_tooltip =
        String::from("if omitted then the current topic is requested");

    if let Some(max_len) = max_len {
        topic_tooltip.push_str(format!("\nmaximum length: {max_len}").as_str());
    }

    Command {
        title: "TOPIC",
        args: vec![
            Argument {
                text: "channel",
                kind: if default.is_some() {
                    ArgumentKind::Optional { skipped: false }
                } else {
                    ArgumentKind::Required
                },
                tooltip: default.map(|default| {
                    format!("may be skipped (default: {default})")
                }),
            },
            Argument {
                text: "topic",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: Some(topic_tooltip),
            },
        ],
        subcommands: None,
    }
}

static USERIP_COMMAND: LazyLock<Command> = LazyLock::new(|| Command {
    title: "USERIP",
    args: vec![Argument {
        text: "nickname",
        kind: ArgumentKind::Required,
        tooltip: None,
    }],
    subcommands: None,
});

fn whox_command() -> Command {
    Command {
        title: "WHO",
        args: vec![
            Argument {
                text: "target",
                kind: ArgumentKind::Required,
                tooltip: None,
            },
            Argument {
                text: "fields",
                kind: ArgumentKind::Optional { skipped: false },
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
            Argument {
                text: "token",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: Some(String::from("1-3 digits")),
            },
        ],
        subcommands: None,
    }
}

fn who_command() -> Command {
    Command {
        title: "WHO",
        args: vec![Argument {
            text: "target",
            kind: ArgumentKind::Required,
            tooltip: None,
        }],
        subcommands: None,
    }
}

fn whois_command(target_limit: Option<u16>) -> Command {
    let mut nicks_tooltip = String::from("comma-separated");

    let nicks_text = if let Some(target_limit) = target_limit {
        nicks_tooltip.push_str(format!("\nup to {target_limit} nick").as_str());
        if target_limit != 1 {
            nicks_tooltip.push('s');
            "nicks"
        } else {
            "nick"
        }
    } else {
        "nick"
    };

    Command {
        title: "WHOIS",
        args: vec![
            Argument {
                text: "server",
                kind: ArgumentKind::Optional { skipped: false },
                tooltip: Some(String::from(
                    "may be skipped (default: the connected server)",
                )),
            },
            Argument {
                text: nicks_text,
                kind: ArgumentKind::Required,
                tooltip: Some(nicks_tooltip),
            },
        ],
        subcommands: None,
    }
}

#[derive(Debug, Clone, Default)]
enum Emojis {
    #[default]
    Idle,
    Selecting {
        highlighted: Option<usize>,
        filtered: Vec<&'static str>,
    },
    Selected {
        emoji: &'static str,
    },
}

impl Emojis {
    fn process(&mut self, input_shortcode: &str, config: &Config) {
        let input_shortcode = input_shortcode.strip_prefix(":").unwrap_or("");

        if input_shortcode.len()
            < config.buffer.emojis.characters_to_trigger_picker
        {
            *self = Self::default();
            return;
        }

        if let Some(shortcode) = config
            .buffer
            .emojis
            .auto_replace
            .then(|| input_shortcode.strip_suffix(":"))
            .flatten()
            .map(str::to_lowercase)
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

        let input_shortcode = input_shortcode
            .strip_suffix(":")
            .unwrap_or(input_shortcode)
            .to_lowercase();

        let mut filtered = emojis::iter()
            .flat_map(|emoji| {
                emoji.shortcodes().filter_map(|shortcode| {
                    if shortcode.contains(&input_shortcode) {
                        Some(FilteredShortcode {
                            similarity: jaro_winkler(
                                &input_shortcode,
                                shortcode,
                            ),
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
            && let Some(shortcode) = filtered.get(*index).copied()
        {
            *self = Self::Idle;

            return pick_emoji(shortcode, config.buffer.emojis.skin_tone)
                .map(ToString::to_string);
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

fn replace_word_with_text(
    input: &str,
    cursor_position: usize,
    text: &str,
    suffix: Option<&str>,
) -> Vec<text_editor::Action> {
    let mut actions: Vec<text_editor::Action> = vec![];

    let append_suffix = if cursor_position == input.len() {
        if let Some((last_word_position, last_word)) = input
            .split(' ')
            .rev()
            .enumerate()
            .find(|(_, word)| !word.is_empty())
        {
            actions.extend(iter::repeat_n(
                text_editor::Action::Select(text_editor::Motion::Left),
                last_word_position + last_word.len(),
            ));
        }

        true
    } else {
        let mut previous_word_bounds = Option::<RangeInclusive<usize>>::None;

        let mut append_suffix = false;

        for word in input.split(' ') {
            let word_bounds =
                if let Some(previous_word_bounds) = previous_word_bounds {
                    RangeInclusive::new(
                        previous_word_bounds.end() + 1,
                        previous_word_bounds.end() + 1 + word.len(),
                    )
                } else {
                    RangeInclusive::new(0, word.len())
                };

            if word_bounds.contains(&cursor_position) {
                if (cursor_position - word_bounds.start())
                    <= (word_bounds.end() - cursor_position)
                {
                    actions.extend(iter::repeat_n(
                        text_editor::Action::Move(text_editor::Motion::Left),
                        cursor_position - word_bounds.start(),
                    ));

                    actions.extend(iter::repeat_n(
                        text_editor::Action::Select(text_editor::Motion::Right),
                        word_bounds.end() - word_bounds.start(),
                    ));
                } else {
                    actions.extend(iter::repeat_n(
                        text_editor::Action::Move(text_editor::Motion::Right),
                        word_bounds.end() - cursor_position,
                    ));

                    actions.extend(iter::repeat_n(
                        text_editor::Action::Select(text_editor::Motion::Left),
                        word_bounds.end() - word_bounds.start(),
                    ));
                }

                if let Some(suffix) = suffix {
                    append_suffix = input.get(*word_bounds.end()..).is_none_or(
                        |after_word| !after_word.starts_with(suffix),
                    );
                }

                break;
            }

            previous_word_bounds = Some(word_bounds);
        }

        append_suffix
    };

    actions.push(text_editor::Action::Edit(text_editor::Edit::Paste(
        std::sync::Arc::new(text.to_string()),
    )));

    if let Some(suffix) = suffix
        && append_suffix
    {
        actions.push(text_editor::Action::Edit(text_editor::Edit::Paste(
            std::sync::Arc::new(suffix.to_string()),
        )));
    }

    actions
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

fn get_word(input: &str, cursor_position: usize) -> Option<&str> {
    let mut previous_word_bounds = Option::<RangeInclusive<usize>>::None;

    if cursor_position == input.len() {
        return input.split(' ').rfind(|word| !word.is_empty());
    }

    for word in input.split(' ') {
        let word_bounds =
            if let Some(previous_word_bounds) = previous_word_bounds {
                RangeInclusive::new(
                    previous_word_bounds.end() + 1,
                    previous_word_bounds.end() + 1 + word.len(),
                )
            } else {
                RangeInclusive::new(0, word.len())
            };

        if word_bounds.contains(&cursor_position) {
            return (!word.is_empty()).then_some(word);
        }

        previous_word_bounds = Some(word_bounds);
    }

    None
}
