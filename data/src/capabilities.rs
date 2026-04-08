use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::string::ToString;
use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use irc::proto::{self, Tags, command, format};

use crate::message::formatting::{Modifier, update_formatting_with_modifier};
use crate::{Target, User, config, message};

pub static DEFAULT: LazyLock<Capabilities> =
    LazyLock::new(Capabilities::default);

// This is not an exhaustive list of IRCv3 capabilities, just the ones that
// Halloy will request when available.  When adding new IRCv3 capabilities to
// Halloy they should be added to this enum (Capability), Capability::from_str,
// and Capabilities::create_requested.
#[derive(Debug, Eq, PartialEq, Hash)]
pub enum Capability {
    AccountNotify,
    AwayNotify,
    Batch,
    BouncerNetworks,
    Chathistory,
    Chghost,
    EchoMessage,
    EventPlayback,
    ExtendedJoin,
    ExtendedMonitor,
    InviteNotify,
    LabeledResponse,
    MessageTags,
    MessageRedaction,
    Multiline,
    MultiPrefix,
    Metadata,
    ReadMarker,
    Sasl,
    ServerTime,
    Setname,
    UserhostInNames,
}

impl FromStr for Capability {
    type Err = &'static str;

    fn from_str(cap: &str) -> Result<Self, Self::Err> {
        match cap {
            "account-notify" => Ok(Self::AccountNotify),
            "away-notify" => Ok(Self::AwayNotify),
            "batch" => Ok(Self::Batch),
            "chghost" => Ok(Self::Chghost),
            "draft/chathistory" => Ok(Self::Chathistory),
            "draft/event-playback" => Ok(Self::EventPlayback),
            "draft/multiline" => Ok(Self::Multiline),
            "draft/read-marker" => Ok(Self::ReadMarker),
            "echo-message" => Ok(Self::EchoMessage),
            "extended-join" => Ok(Self::ExtendedJoin),
            "extended-monitor" => Ok(Self::ExtendedMonitor),
            "invite-notify" => Ok(Self::InviteNotify),
            "labeled-response" => Ok(Self::LabeledResponse),
            "message-tags" => Ok(Self::MessageTags),
            "draft/message-redaction" => Ok(Self::MessageRedaction),
            "multi-prefix" => Ok(Self::MultiPrefix),
            "draft/metadata-2" => Ok(Self::Metadata),
            "server-time" => Ok(Self::ServerTime),
            "setname" => Ok(Self::Setname),
            "soju.im/bouncer-networks" => Ok(Self::BouncerNetworks),
            "userhost-in-names" => Ok(Self::UserhostInNames),
            _ if cap.starts_with("sasl") => Ok(Self::Sasl),
            _ => Err("unknown capability"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MultilineLimits {
    pub max_bytes: usize,
    pub max_lines: Option<usize>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CapParseError {
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Missing key `{0}` in dictionary: {1}")]
    MissingKey(String, String),
}

impl FromStr for MultilineLimits {
    type Err = CapParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dictionary = s
            .split(',')
            .flat_map(|s| s.split_once('='))
            .collect::<HashMap<_, _>>();

        Ok(MultilineLimits {
            max_bytes: dictionary
                .get("max-bytes")
                .ok_or_else(|| {
                    CapParseError::MissingKey(
                        "max-bytes".to_owned(),
                        s.to_owned(),
                    )
                })?
                .parse::<usize>()?,
            max_lines: dictionary
                .get("max-lines")
                .map(|s| s.parse::<usize>())
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MetadataLimits {
    pub max_subs: usize,
}

impl FromStr for MetadataLimits {
    type Err = CapParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dictionary = s
            .split(',')
            .flat_map(|s| s.split_once('='))
            .collect::<HashMap<_, _>>();

        Ok(MetadataLimits {
            max_subs: dictionary
                .get("max-subs")
                .ok_or_else(|| {
                    CapParseError::MissingKey(
                        "max-subs".to_owned(),
                        s.to_owned(),
                    )
                })?
                .parse::<usize>()?,
        })
    }
}

impl MultilineLimits {
    pub fn concat_bytes(
        &self,
        relay_bytes: usize,
        batch_kind: MultilineBatchKind,
        target: &str,
    ) -> usize {
        // Message byte limit - relay bytes - space - command - space - target - message separator - crlf
        format::BYTE_LIMIT.saturating_sub(
            match batch_kind {
                MultilineBatchKind::PRIVMSG => 7,
                MultilineBatchKind::NOTICE => 6,
            } + target.len()
                + relay_bytes
                + 6,
        )
    }
}

// Forbid splitting inside formatting sequences and attempt to split at spaces
// for better compatibility with clients that don't support multiline.
pub fn multiline_concat_lines(concat_bytes: usize, text: &str) -> Vec<&str> {
    let mut lines = Vec::new();

    let mut line_start = 0;
    let mut last_space = 0;
    let mut line_bytes = 0;

    let mut modifiers = HashSet::new();
    let mut fg = None;
    let mut bg = None;

    let mut iter = text.chars().peekable();

    while let Some(c) = iter.next() {
        let sequence_bytes = if let Ok(modifier) = Modifier::try_from(c) {
            let (sequence_bytes, comma) = update_formatting_with_modifier(
                &mut modifiers,
                &mut fg,
                &mut bg,
                modifier,
                &mut iter,
            );

            // This will prevent breaking a color modifier away from a
            // non-modifier, trailing comma; behaves that way solely for
            // simplicity's sake
            sequence_bytes + comma.map_or(0, char::len_utf8)
        } else {
            c.len_utf8()
        };

        if (line_bytes + sequence_bytes) > concat_bytes {
            if last_space > line_start {
                lines.push(&text[line_start..last_space + ' '.len_utf8()]);

                line_bytes -= last_space + ' '.len_utf8() - line_start;
                line_start = last_space + ' '.len_utf8();

                if line_bytes > concat_bytes {
                    lines.push(&text[line_start..line_start + line_bytes]);

                    line_start += line_bytes;
                    line_bytes = 0;
                }
            } else {
                lines.push(&text[line_start..line_start + line_bytes]);

                line_start += line_bytes;
                line_bytes = 0;
            }
        }

        if c == ' ' {
            last_space = line_start + line_bytes;
        }

        line_bytes += sequence_bytes;
    }

    lines.push(&text[line_start..]);

    lines
}

pub fn multiline_encoded(
    user: Option<&User>,
    batch_kind: MultilineBatchKind,
    target: &Target,
    text: &str,
    tags: Tags,
) -> message::Encoded {
    let mut encoded = command!(
        match batch_kind {
            MultilineBatchKind::PRIVMSG => "PRIVMSG",
            MultilineBatchKind::NOTICE => "NOTICE",
        },
        target.as_str(),
        text,
    );

    if let Some(user) = user {
        encoded.source = Some(proto::Source::User(proto::User {
            nickname: user.nickname().to_string(),
            username: user.username().map(ToString::to_string),
            hostname: user.hostname().map(ToString::to_string),
        }));
    }

    encoded.tags = tags;

    message::Encoded(encoded)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MultilineBatchKind {
    PRIVMSG,
    NOTICE,
}

#[derive(Debug, Default)]
pub struct Capabilities {
    listed: HashMap<String, String>,
    pending: HashMap<String, String>,
    acknowledged: HashSet<Capability>,
}

impl Capabilities {
    pub fn acknowledge(&mut self, caps: impl Iterator<Item = String>) {
        for cap in caps {
            if let Ok(cap) = Capability::from_str(cap.as_str()) {
                self.acknowledged.insert(cap);
            }
        }
    }

    pub fn acknowledged(&self, cap: Capability) -> bool {
        self.acknowledged.contains(&cap)
    }

    pub fn create_requested(
        &mut self,
        config: &config::Server,
    ) -> Vec<&'static str> {
        let mut requested = vec![];

        if self.pending.contains_key("invite-notify")
            && !self.acknowledged(Capability::InviteNotify)
        {
            requested.push("invite-notify");
        }

        if self.pending.contains_key("userhost-in-names")
            && !self.acknowledged(Capability::UserhostInNames)
        {
            requested.push("userhost-in-names");
        }

        if self.pending.contains_key("away-notify")
            && !self.acknowledged(Capability::AwayNotify)
        {
            requested.push("away-notify");
        }

        if self.pending.contains_key("message-tags")
            && !self.acknowledged(Capability::MessageTags)
        {
            requested.push("message-tags");
        }

        if self.pending.contains_key("draft/message-redaction")
            && !self.acknowledged(Capability::MessageRedaction)
        {
            requested.push("draft/message-redaction");
        }

        if self.pending.contains_key("server-time")
            && !self.acknowledged(Capability::ServerTime)
        {
            requested.push("server-time");
        }

        if self.pending.contains_key("chghost")
            && !self.acknowledged(Capability::Chghost)
        {
            requested.push("chghost");
        }

        if self.pending.contains_key("extended-monitor")
            && !self.acknowledged(Capability::ExtendedMonitor)
        {
            requested.push("extended-monitor");
        }

        if self.pending.contains_key("account-notify")
            || self.acknowledged(Capability::AccountNotify)
        {
            if !self.acknowledged(Capability::AccountNotify) {
                requested.push("account-notify");
            }

            if self.pending.contains_key("extended-join")
                && !self.acknowledged(Capability::ExtendedJoin)
            {
                requested.push("extended-join");
            }
        }

        if self.pending.contains_key("batch")
            || self.acknowledged(Capability::Batch)
        {
            if !self.acknowledged(Capability::Batch) {
                requested.push("batch");
            }

            // We require batch for chathistory support
            if (self.pending.contains_key("draft/chathistory")
                && config.chathistory)
                || self.acknowledged(Capability::Chathistory)
            {
                if !self.acknowledged(Capability::Chathistory) {
                    requested.push("draft/chathistory");
                }

                if self.pending.contains_key("draft/event-playback")
                    && !self.acknowledged(Capability::EventPlayback)
                {
                    requested.push("draft/event-playback");
                }
            }
        }

        if self.pending.contains_key("labeled-response")
            && !self.acknowledged(Capability::LabeledResponse)
        {
            requested.push("labeled-response");
        }

        if self.pending.contains_key("echo-message")
            && !self.acknowledged(Capability::EchoMessage)
        {
            requested.push("echo-message");
        }

        if self.pending.contains_key("multi-prefix")
            && !self.acknowledged(Capability::MultiPrefix)
        {
            requested.push("multi-prefix");
        }

        if self.pending.contains_key("draft/read-marker")
            && !self.acknowledged(Capability::ReadMarker)
        {
            requested.push("draft/read-marker");
        }

        if self.pending.contains_key("setname")
            && !self.acknowledged(Capability::Setname)
        {
            requested.push("setname");
        }

        if self.pending.contains_key("soju.im/bouncer-networks")
            && !self.acknowledged(Capability::BouncerNetworks)
        {
            requested.push("soju.im/bouncer-networks");
        }

        if self.pending.iter().any(|(cap, _)| cap.starts_with("sasl"))
            && !self.acknowledged(Capability::Sasl)
        {
            requested.push("sasl");
        }

        if let Some(multiline) = self.pending.get("draft/multiline")
            && !self.acknowledged(Capability::Multiline)
            && MultilineLimits::from_str(multiline).is_ok()
        {
            requested.push("draft/multiline");
        }

        if self.pending.contains_key("draft/metadata-2")
            && !self.acknowledged(Capability::Metadata)
        {
            requested.push("draft/metadata-2");
        }

        for (cap, val) in self.pending.drain() {
            self.listed.insert(cap, val);
        }

        requested
    }

    pub fn delete(&mut self, caps: impl Iterator<Item = String>) {
        for cap in caps {
            if let Ok(cap) = Capability::from_str(cap.as_str()) {
                self.acknowledged.remove(&cap);
            }

            self.listed.remove(&cap);
        }
    }

    pub fn extend_list<'a>(&mut self, caps: impl Iterator<Item = &'a str>) {
        for cap in caps {
            if let Some((left, right)) = cap.split_once('=') {
                self.pending.insert(left.to_string(), right.to_string());
            } else {
                self.pending.insert(cap.to_string(), String::new());
            }
        }
    }

    pub fn multiline_limits(&self) -> Option<MultilineLimits> {
        self.acknowledged(Capability::Multiline)
            .then(|| {
                MultilineLimits::from_str(self.listed.get("draft/multiline")?)
                    .ok()
            })
            .flatten()
    }

    pub fn metadata_limits(&self) -> Option<MetadataLimits> {
        self.acknowledged(Capability::Metadata)
            .then(|| {
                MetadataLimits::from_str(self.listed.get("draft/metadata-2")?)
                    .ok()
            })
            .flatten()
    }

    pub fn contains_multiline_limits(&self) -> bool {
        self.multiline_limits().is_some()
    }
}

#[derive(Debug, Clone)]
pub struct LabeledResponseContext {
    pub label_as_id: message::Id,
    pub server_time: DateTime<Utc>,
}

impl LabeledResponseContext {
    pub fn new(message: &message::Encoded, label: &str) -> Self {
        Self {
            // Prefix ':' to ensure it cannot match any valid message id
            label_as_id: format!(":label={label}").into(),
            server_time: message.server_time_or_now(),
        }
    }
}
