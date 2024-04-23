// ISUPPORT Parameter References
// - https://defs.ircdocs.horse/defs/isupport.html
// - https://modern.ircdocs.horse/#rplisupport-005
// - https://ircv3.net/specs/extensions/chathistory
// - https://ircv3.net/specs/extensions/monitor
// - https://ircv3.net/specs/extensions/utf8-only
// - https://ircv3.net/specs/extensions/whox
// - https://github.com/ircv3/ircv3-specifications/pull/464/files
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum Parameter {
    ACCEPT(u16),
    ACCOUNTEXTBAN(Vec<String>),
    AWAYLEN(Option<u16>),
    BOT(char),
    CALLERID(char),
    CASEMAPPING(CaseMap),
    CHANLIMIT(Vec<ChannelLimit>),
    CHANMODES(Vec<ChannelMode>),
    CHANNELLEN(u16),
    CHANTYPES(Option<String>),
    CHATHISTORY(u16),
    CLIENTTAGDENY(Vec<ClientOnlyTags>),
    CLIENTVER(u16, u16),
    CNOTICE,
    CPRIVMSG,
    DEAF(char),
    ELIST(String),
    ESILENCE(Option<String>),
    ETRACE,
    EXCEPTS(char),
    EXTBAN(Option<char>, String),
    FNC,
    HOSTLEN(u16),
    INVEX(char),
    KEYLEN(u16),
    KICKLEN(u16),
    KNOCK,
    LINELEN(u16),
    MAP,
    MAXBANS(u16),
    MAXCHANNELS(u16),
    MAXLIST(Vec<ModesLimit>),
    MAXPARA(u16),
    MAXTARGETS(Option<u16>),
    METADATA(Option<u16>),
    MODES(Option<u16>),
    MONITOR(Option<u16>),
    MSGREFTYPES(Vec<MessageReferenceType>),
    NAMESX,
    NETWORK(String),
    NICKLEN(u16),
    OVERRIDE,
    PREFIX(Vec<PrefixMap>),
    SAFELIST,
    SECURELIST,
    SILENCE(Option<u16>),
    STATUSMSG(String),
    TARGMAX(Vec<CommandTargetLimit>),
    TOPICLEN(u16),
    UHNAMES,
    USERIP,
    USERLEN(u16),
    UTF8ONLY,
    VLIST(String),
    WATCH(u16),
    WHOX,
    Negation(String),
}

impl TryFrom<String> for Parameter {
    type Error = &'static str;

    fn try_from(isupport: String) -> Result<Self, Self::Error> {
        Self::try_from(isupport.as_str())
    }
}

impl<'a> TryFrom<&'a str> for Parameter {
    type Error = &'static str;

    fn try_from(isupport: &'a str) -> Result<Self, Self::Error> {
        if isupport.is_empty() {
            return Err("empty ISUPPORT parameter not allowed");
        }

        match isupport.chars().next() {
            Some('-') => Ok(Parameter::Negation(isupport[1..].to_string())),
            _ => {
                if let Some((parameter, value)) = isupport.split_once('=') {
                    match parameter {
                        "ACCEPT" => Ok(Parameter::ACCEPT(parse_required_positive_integer(value)?)),
                        "ACCOUNTEXTBAN" => {
                            let account_based_extended_ban_masks =
                                value.split(',').map(String::from).collect::<Vec<_>>();

                            if !account_based_extended_ban_masks.is_empty() {
                                Ok(Parameter::ACCOUNTEXTBAN(account_based_extended_ban_masks))
                            } else {
                                Err("no valid account-based extended ban masks")
                            }
                        }
                        "AWAYLEN" => {
                            Ok(Parameter::AWAYLEN(parse_optional_positive_integer(value)?))
                        }
                        "BOT" => Ok(Parameter::BOT(parse_required_letter(value, None)?)),
                        "CALLERID" => Ok(Parameter::CALLERID(parse_required_letter(
                            value,
                            Some(default_caller_id_letter()),
                        )?)),
                        "CASEMAPPING" => match value {
                            "ascii" => Ok(Parameter::CASEMAPPING(CaseMap::ASCII)),
                            "rfc1459" => Ok(Parameter::CASEMAPPING(CaseMap::RFC1459)),
                            "rfc1459-strict" => Ok(Parameter::CASEMAPPING(CaseMap::RFC1459_STRICT)),
                            "rfc7613" => Ok(Parameter::CASEMAPPING(CaseMap::RFC7613)),
                            _ => Err("unknown casemapping"),
                        },
                        "CHANLIMIT" => {
                            let mut channel_limits = vec![];

                            value.split(',').for_each(|channel_limit| {
                                if let Some((prefix, limit)) = channel_limit.split_once(':') {
                                    if limit.is_empty() {
                                        channel_limits.push(ChannelLimit {
                                            prefix: prefix.to_string(),
                                            limit: None,
                                        });
                                    } else if let Ok(limit) = limit.parse::<u16>() {
                                        channel_limits.push(ChannelLimit {
                                            prefix: prefix.to_string(),
                                            limit: Some(limit),
                                        });
                                    }
                                }
                            });

                            if !channel_limits.is_empty() {
                                Ok(Parameter::CHANLIMIT(channel_limits))
                            } else {
                                Err("no valid channel limits")
                            }
                        }
                        "CHANMODES" => {
                            let mut channel_modes = vec![];

                            ('A'..='Z')
                                .zip(value.split(','))
                                .for_each(|(letter, modes)| {
                                    channel_modes.push(ChannelMode {
                                        letter,
                                        modes: String::from(modes),
                                    })
                                });

                            if !channel_modes.is_empty() {
                                Ok(Parameter::CHANMODES(channel_modes))
                            } else {
                                Err("no valid channel modes")
                            }
                        }
                        "CHANNELLEN" => Ok(Parameter::CHANNELLEN(parse_required_positive_integer(
                            value,
                        )?)),
                        "CHANTYPES" => Ok(Parameter::CHANTYPES(parse_optional_string(value))),
                        "CHATHISTORY" => Ok(Parameter::CHATHISTORY(
                            parse_required_positive_integer(value)?,
                        )),
                        "CLIENTTAGDENY" => {
                            let mut client_tag_denials = vec![];

                            value
                                .split(',')
                                .for_each(|client_tag_denial| {
                                    match client_tag_denial.chars().next() {
                                        Some('*') => {
                                            client_tag_denials.push(ClientOnlyTags::DenyAll)
                                        }
                                        Some('-') => {
                                            client_tag_denials.push(ClientOnlyTags::Allowed(
                                                client_tag_denial[1..].to_string(),
                                            ))
                                        }
                                        _ => client_tag_denials.push(ClientOnlyTags::Denied(
                                            client_tag_denial.to_string(),
                                        )),
                                    }
                                });

                            if !client_tag_denials.is_empty() {
                                Ok(Parameter::CLIENTTAGDENY(client_tag_denials))
                            } else {
                                Err("no valid client tag denials")
                            }
                        }
                        "CLIENTVER" => {
                            if let Some((major, minor)) = value.split_once('.') {
                                if let (Ok(major), Ok(minor)) =
                                    (major.parse::<u16>(), minor.parse::<u16>())
                                {
                                    return Ok(Parameter::CLIENTVER(major, minor));
                                }
                            }

                            Err("value must be a <major>.<minor> version number")
                        }
                        "CNOTICE" => Ok(Parameter::CNOTICE),
                        "CPRIVMSG" => Ok(Parameter::CPRIVMSG),
                        "DEAF" => Ok(Parameter::DEAF(parse_required_letter(
                            value,
                            Some(default_deaf_letter()),
                        )?)),
                        "ELIST" => Ok(Parameter::ELIST(parse_required_string(value)?)),
                        "ESILENCE" => Ok(Parameter::ESILENCE(parse_optional_string(value))),
                        "ETRACE" => Ok(Parameter::ETRACE),
                        "EXCEPTS" => Ok(Parameter::EXCEPTS(parse_required_letter(
                            value,
                            Some(default_ban_exception_channel_letter()),
                        )?)),
                        "EXTBAN" => {
                            if let Some((prefix, types)) = value.split_once(',') {
                                if prefix.is_empty() {
                                    Ok(Parameter::EXTBAN(None, types.to_string()))
                                } else {
                                    Ok(Parameter::EXTBAN(prefix.chars().next(), types.to_string()))
                                }
                            } else {
                                Err("no valid extended ban masks")
                            }
                        }
                        "FNC" => Ok(Parameter::FNC),
                        "HOSTLEN" => {
                            Ok(Parameter::HOSTLEN(parse_required_positive_integer(value)?))
                        }
                        "INVEX" => Ok(Parameter::INVEX(parse_required_letter(
                            value,
                            Some(default_invite_exception_letter()),
                        )?)),
                        "KEYLEN" => Ok(Parameter::KEYLEN(parse_required_positive_integer(value)?)),
                        "KICKLEN" => {
                            Ok(Parameter::KICKLEN(parse_required_positive_integer(value)?))
                        }
                        "KNOCK" => Ok(Parameter::KNOCK),
                        "LINELEN" => {
                            Ok(Parameter::LINELEN(parse_required_positive_integer(value)?))
                        }
                        "MAP" => Ok(Parameter::MAP),
                        "MAXBANS" => {
                            Ok(Parameter::MAXBANS(parse_required_positive_integer(value)?))
                        }
                        "MAXCHANNELS" => Ok(Parameter::MAXCHANNELS(
                            parse_required_positive_integer(value)?,
                        )),
                        "MAXLIST" => {
                            let mut modes_limits = vec![];

                            value.split(',').for_each(|modes_limit| {
                                if let Some((modes, limit)) = modes_limit.split_once(':') {
                                    if let Ok(limit) = limit.parse::<u16>() {
                                        modes_limits.push(ModesLimit {
                                            modes: modes.to_string(),
                                            limit,
                                        });
                                    }
                                }
                            });

                            if !modes_limits.is_empty() {
                                Ok(Parameter::MAXLIST(modes_limits))
                            } else {
                                Err("no valid modes limits")
                            }
                        }
                        "MAXPARA" => {
                            Ok(Parameter::MAXPARA(parse_required_positive_integer(value)?))
                        }
                        "MAXTARGETS" => Ok(Parameter::MAXTARGETS(parse_optional_positive_integer(
                            value,
                        )?)),
                        "METADATA" => {
                            Ok(Parameter::METADATA(parse_optional_positive_integer(value)?))
                        }
                        "MODES" => Ok(Parameter::MODES(parse_optional_positive_integer(value)?)),
                        "MONITOR" => {
                            Ok(Parameter::MONITOR(parse_optional_positive_integer(value)?))
                        }
                        "MSGREFTYPES" => {
                            let mut message_reference_types = vec![];

                            value.split(',').for_each(|message_reference_type| {
                                match message_reference_type {
                                    "msgid" => message_reference_types
                                        .insert(0, MessageReferenceType::MessageID),
                                    "timestamp" => message_reference_types
                                        .insert(0, MessageReferenceType::Timestamp),
                                    _ => (),
                                }
                            });

                            Ok(Parameter::MSGREFTYPES(message_reference_types))
                        }
                        "NAMESX" => Ok(Parameter::NAMESX),
                        "NETWORK" => Ok(Parameter::NETWORK(value.to_string())),
                        "NICKLEN" | "MAXNICKLEN" => {
                            Ok(Parameter::NICKLEN(parse_required_positive_integer(value)?))
                        }
                        "OVERRIDE" => Ok(Parameter::OVERRIDE),
                        "PREFIX" => {
                            let mut prefix_maps = vec![];

                            if let Some((modes, prefixes)) = value.split_once(')') {
                                let modes = &modes[1..];

                                modes
                                    .chars()
                                    .zip(prefixes.chars())
                                    .for_each(|(mode, prefix)| {
                                        prefix_maps.push(PrefixMap { mode, prefix })
                                    });

                                Ok(Parameter::PREFIX(prefix_maps))
                            } else {
                                Err("unrecognized PREFIX format")
                            }
                        }
                        "SAFELIST" => Ok(Parameter::SAFELIST),
                        "SECURELIST" => Ok(Parameter::SECURELIST),
                        "SILENCE" => {
                            Ok(Parameter::SILENCE(parse_optional_positive_integer(value)?))
                        }
                        "STATUSMSG" => Ok(Parameter::STATUSMSG(value.to_string())),
                        "TARGMAX" => {
                            let mut command_target_limits = vec![];

                            value.split(',').for_each(|command_target_limit| {
                                if let Some((command, limit)) = command_target_limit.split_once(':')
                                {
                                    if limit.is_empty() {
                                        command_target_limits.push(CommandTargetLimit {
                                            command: command.to_string(),
                                            limit: None,
                                        });
                                    } else if let Ok(limit) = limit.parse::<u16>() {
                                        command_target_limits.push(CommandTargetLimit {
                                            command: command.to_string(),
                                            limit: Some(limit),
                                        });
                                    }
                                }
                            });

                            if !command_target_limits.is_empty() {
                                Ok(Parameter::TARGMAX(command_target_limits))
                            } else {
                                Err("no valid command target limits")
                            }
                        }
                        "TOPICLEN" => {
                            Ok(Parameter::TOPICLEN(parse_required_positive_integer(value)?))
                        }
                        "UHNAMES" => Ok(Parameter::UHNAMES),
                        "USERIP" => Ok(Parameter::USERIP),
                        "USERLEN" => {
                            Ok(Parameter::USERLEN(parse_required_positive_integer(value)?))
                        }
                        "UTF8ONLY" => Ok(Parameter::UTF8ONLY),
                        "VLIST" => Ok(Parameter::VLIST(parse_required_string(value)?)),
                        "WATCH" => Ok(Parameter::WATCH(parse_required_positive_integer(value)?)),
                        "WHOX" => Ok(Parameter::WHOX),
                        _ => Err("unknown ISUPPORT parameter"),
                    }
                } else {
                    match isupport {
                        "ACCEPT" => Err("value required"),
                        "ACCOUNTEXTBAN" => Err("value(s) required"),
                        "AWAYLEN" => Ok(Parameter::AWAYLEN(None)),
                        "BOT" => Err("value required"),
                        "CALLERID" => Ok(Parameter::CALLERID(default_caller_id_letter())),
                        "CASEMAPPING" => Err("value required"),
                        "CHANLIMIT" => Err("value(s) required"),
                        "CHANMODES" => Err("value(s) required"),
                        "CHANNELLEN" => Err("value required"),
                        "CHANTYPES" => Ok(Parameter::CHANTYPES(None)),
                        "CHATHISTORY" => Err("value required"),
                        "CLIENTTAGDENY" => Err("value(s) required"),
                        "CLIENTVER" => Err("value required"),
                        "DEAF" => Ok(Parameter::DEAF(default_deaf_letter())),
                        "ELIST" => Err("value required"),
                        "ESILENCE" => Ok(Parameter::ESILENCE(None)),
                        "ETRACE" => Ok(Parameter::ETRACE),
                        "EXCEPTS" => Ok(Parameter::EXCEPTS(default_ban_exception_channel_letter())),
                        "EXTBAN" => Err("value required"),
                        "FNC" => Ok(Parameter::FNC),
                        "HOSTLEN" => Err("value required"),
                        "INVEX" => Ok(Parameter::INVEX(default_invite_exception_letter())),
                        "KEYLEN" => Err("value required"),
                        "KICKLEN" => Err("value required"),
                        "KNOCK" => Ok(Parameter::KNOCK),
                        "LINELEN" => Err("value required"),
                        "MAP" => Ok(Parameter::MAP),
                        "MAXBANS" => Err("value required"),
                        "MAXCHANNELS" => Err("value required"),
                        "MAXLIST" => Err("value(s) required"),
                        "MAXPARA" => Err("value required"),
                        "MAXTARGETS" => Ok(Parameter::MAXTARGETS(None)),
                        "METADATA" => Ok(Parameter::METADATA(None)),
                        "MODES" => Ok(Parameter::MODES(None)),
                        "MONITOR" => Ok(Parameter::MONITOR(None)),
                        "MSGREFTYPES" => Ok(Parameter::MSGREFTYPES(vec![])),
                        "NAMESX" => Ok(Parameter::NAMESX),
                        "NETWORK" => Err("value required"),
                        "NICKLEN" | "MAXNICKLEN" => Err("value required"),
                        "OVERRIDE" => Ok(Parameter::OVERRIDE),
                        "PREFIX" => Ok(Parameter::PREFIX(vec![])),
                        "SAFELIST" => Ok(Parameter::SAFELIST),
                        "SECURELIST" => Ok(Parameter::SECURELIST),
                        "SILENCE" => Ok(Parameter::SILENCE(None)),
                        "STATUSMSG" => Err("value required"),
                        "TARGMAX" => Ok(Parameter::TARGMAX(vec![])),
                        "TOPICLEN" => Err("value required"),
                        "UHNAMES" => Ok(Parameter::UHNAMES),
                        "USERIP" => Ok(Parameter::USERIP),
                        "USERLEN" => Err("value required"),
                        "UTF8ONLY" => Ok(Parameter::UTF8ONLY),
                        "VLIST" => Err("value required"),
                        "WATCH" => Err("value required"),
                        "WHOX" => Ok(Parameter::WHOX),
                        _ => Err("unknown ISUPPORT parameter"),
                    }
                }
            }
        }
    }
}

impl Parameter {
    pub fn key(&self) -> &str {
        match self {
            Parameter::ACCEPT(_) => "ACCEPT",
            Parameter::ACCOUNTEXTBAN(_) => "ACCOUNTEXTBAN",
            Parameter::AWAYLEN(_) => "AWAYLEN",
            Parameter::BOT(_) => "BOT",
            Parameter::CALLERID(_) => "CALLERID",
            Parameter::CASEMAPPING(_) => "CASEMAPPING",
            Parameter::CHANLIMIT(_) => "CHANLIMIT",
            Parameter::CHANMODES(_) => "CHANMODES",
            Parameter::CHANNELLEN(_) => "CHANNELLEN",
            Parameter::CHANTYPES(_) => "CHANTYPES",
            Parameter::CHATHISTORY(_) => "CHATHISTORY",
            Parameter::CLIENTTAGDENY(_) => "CLIENTTAGDENY",
            Parameter::CLIENTVER(_, _) => "CLIENTVER",
            Parameter::CNOTICE => "CNOTICE",
            Parameter::CPRIVMSG => "CPRIVMSG",
            Parameter::DEAF(_) => "DEAF",
            Parameter::ELIST(_) => "ELIST",
            Parameter::ESILENCE(_) => "ESILENCE",
            Parameter::ETRACE => "ETRACE",
            Parameter::EXCEPTS(_) => "EXCEPTS",
            Parameter::EXTBAN(_, _) => "EXTBAN",
            Parameter::FNC => "FNC",
            Parameter::HOSTLEN(_) => "HOSTLEN",
            Parameter::INVEX(_) => "INVEX",
            Parameter::KEYLEN(_) => "KEYLEN",
            Parameter::KICKLEN(_) => "KICKLEN",
            Parameter::KNOCK => "KNOCK",
            Parameter::LINELEN(_) => "LINELEN",
            Parameter::MAP => "MAP",
            Parameter::MAXBANS(_) => "MAXBANS",
            Parameter::MAXCHANNELS(_) => "MAXCHANNELS",
            Parameter::MAXLIST(_) => "MAXLIST",
            Parameter::MAXPARA(_) => "MAXPARA",
            Parameter::MAXTARGETS(_) => "MAXTARGETS",
            Parameter::METADATA(_) => "METADATA",
            Parameter::MODES(_) => "MODES",
            Parameter::MONITOR(_) => "MONITOR",
            Parameter::MSGREFTYPES(_) => "MSGREFTYPES",
            Parameter::NAMESX => "NAMESX",
            Parameter::NETWORK(_) => "NETWORK",
            Parameter::NICKLEN(_) => "NICKLEN",
            Parameter::OVERRIDE => "OVERRIDE",
            Parameter::PREFIX(_) => "PREFIX",
            Parameter::SAFELIST => "SAFELIST",
            Parameter::SECURELIST => "SECURELIST",
            Parameter::SILENCE(_) => "SILENCE",
            Parameter::STATUSMSG(_) => "STATUSMSG",
            Parameter::TARGMAX(_) => "TARGMAX",
            Parameter::TOPICLEN(_) => "TOPICLEN",
            Parameter::UHNAMES => "UHNAMES",
            Parameter::USERIP => "USERIP",
            Parameter::USERLEN(_) => "USERLEN",
            Parameter::UTF8ONLY => "UTF8ONLY",
            Parameter::VLIST(_) => "VLIST",
            Parameter::WATCH(_) => "WATCH",
            Parameter::WHOX => "WHOX",
            Parameter::Negation(key) => key.as_ref(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum CaseMap {
    ASCII,
    RFC1459,
    RFC1459_STRICT,
    RFC7613,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ChannelLimit {
    prefix: String,
    limit: Option<u16>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ChannelMode {
    letter: char,
    modes: String,
}

#[derive(Debug)]
pub enum ClientOnlyTags {
    Allowed(String),
    Denied(String),
    DenyAll,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct CommandTargetLimit {
    command: String,
    limit: Option<u16>,
}

#[derive(Debug)]
pub enum MessageReferenceType {
    Timestamp,
    MessageID,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ModesLimit {
    modes: String,
    limit: u16,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PrefixMap {
    prefix: char,
    mode: char,
}

pub fn default_ban_exception_channel_letter() -> char {
    'e'
}

pub fn default_caller_id_letter() -> char {
    'g'
}

pub fn default_deaf_letter() -> char {
    'D'
}

pub fn default_invite_exception_letter() -> char {
    'I'
}

fn parse_optional_positive_integer(value: &str) -> Result<Option<u16>, &'static str> {
    if value.is_empty() {
        Ok(None)
    } else if let Ok(value) = value.parse::<u16>() {
        Ok(Some(value))
    } else {
        Err("optional value must be a positive integer if specified")
    }
}

fn parse_optional_string(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn parse_required_letter(value: &str, default_value: Option<char>) -> Result<char, &'static str> {
    if let Some(value) = value.chars().next() {
        Ok(value)
    } else if let Some(default_value) = default_value {
        Ok(default_value)
    } else {
        Err("value required to be a letter")
    }
}

fn parse_required_positive_integer(value: &str) -> Result<u16, &'static str> {
    if let Ok(value) = value.parse::<u16>() {
        Ok(value)
    } else {
        Err("value required to be a positive integer")
    }
}

fn parse_required_string(value: &str) -> Result<String, &'static str> {
    if !value.is_empty() {
        Ok(value.to_string())
    } else {
        Err("value required")
    }
}
