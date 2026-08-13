use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::string::ToString;
use std::sync::LazyLock;

use chrono::{DateTime, TimeDelta, Utc};
use irc::proto::{self, Tags, command, format};
use serde::Deserialize;

use crate::message::formatting::{Modifier, update_formatting_with_modifier};
use crate::message::time;
use crate::{Target, User, config, message};

pub static DEFAULT: LazyLock<Capabilities> =
    LazyLock::new(Capabilities::default);

// This is not an exhaustive list of IRCv3 capabilities, just the ones that
// Halloy will request when available.  When adding new IRCv3 capabilities to
// Halloy they should be added to this enum (Capability), Capability::from_str,
// and Capabilities::create_requested.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
    NoImplicitNames,
    ReadMarker,
    Relaymsg,
    Sasl,
    ServerTime,
    Setname,
    UserhostInNames,
    Whoami,
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
            "draft/relaymsg" => Ok(Self::Relaymsg),
            "draft/whoami" => Ok(Self::Whoami),
            "echo-message" => Ok(Self::EchoMessage),
            "extended-join" => Ok(Self::ExtendedJoin),
            "extended-monitor" => Ok(Self::ExtendedMonitor),
            "invite-notify" => Ok(Self::InviteNotify),
            "labeled-response" => Ok(Self::LabeledResponse),
            "message-tags" => Ok(Self::MessageTags),
            "draft/message-redaction" => Ok(Self::MessageRedaction),
            "multi-prefix" => Ok(Self::MultiPrefix),
            "draft/metadata-2" => Ok(Self::Metadata),
            "no-implicit-names" => Ok(Self::NoImplicitNames),
            // TODO(quaff): remove `draft/no-implicit-names` support when ergo & soju have both been upgraded
            "draft/no-implicit-names" => Ok(Self::NoImplicitNames),
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
            if let Some(cap) = cap.as_str().strip_prefix('-') {
                if let Ok(cap) = Capability::from_str(cap) {
                    self.acknowledged.remove(&cap);
                }
            } else if let Ok(cap) = Capability::from_str(cap.as_str()) {
                self.acknowledged.insert(cap);
            }
        }
    }

    pub fn acknowledged(&self, cap: Capability) -> bool {
        self.acknowledged.contains(&cap)
    }

    fn create_request(
        &self,
        capability_str: &'static str,
        requirements: &[&'static str],
        available: &HashMap<String, String>,
        config: &config::Server,
    ) -> Option<Cow<'static, str>> {
        let capability_enum = Capability::from_str(capability_str).ok()?;

        if available.contains_key(capability_str)
            && requirements.iter().all(|requirement_str| {
                if let Ok(requirement_enum) =
                    Capability::from_str(requirement_str)
                {
                    (available.contains_key(*requirement_str)
                        || self.acknowledged(requirement_enum))
                        && !config.do_not_request.contains(&requirement_enum)
                } else {
                    false
                }
            })
        {
            match capability_enum {
                Capability::Multiline => {
                    if available.get(capability_str).is_none_or(|multiline| {
                        MultilineLimits::from_str(multiline).is_err()
                    }) {
                        return None;
                    }
                }
                Capability::AccountNotify
                | Capability::AwayNotify
                | Capability::Batch
                | Capability::BouncerNetworks
                | Capability::Chathistory
                | Capability::Chghost
                | Capability::EchoMessage
                | Capability::EventPlayback
                | Capability::ExtendedJoin
                | Capability::ExtendedMonitor
                | Capability::InviteNotify
                | Capability::LabeledResponse
                | Capability::MessageTags
                | Capability::MessageRedaction
                | Capability::MultiPrefix
                | Capability::Metadata
                | Capability::NoImplicitNames
                | Capability::ReadMarker
                | Capability::Relaymsg
                | Capability::Sasl
                | Capability::ServerTime
                | Capability::Setname
                | Capability::UserhostInNames
                | Capability::Whoami => (),
            }

            if !config.do_not_request.contains(&capability_enum)
                && !self.acknowledged(capability_enum)
            {
                return Some(capability_str.into());
            } else if config.do_not_request.contains(&capability_enum)
                && self.acknowledged(capability_enum)
            {
                return Some(format!("-{capability_str}").into());
            }
        }

        None
    }

    fn create_requested(
        &self,
        available: &HashMap<String, String>,
        config: &config::Server,
    ) -> Vec<Cow<'static, str>> {
        let mut requested = vec![];

        if let Some(request) =
            self.create_request("invite-notify", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("userhost-in-names", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("away-notify", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("message-tags", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("draft/relaymsg", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) = self.create_request(
            "draft/message-redaction",
            &[],
            available,
            config,
        ) {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("server-time", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("chghost", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("extended-monitor", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) = self.create_request(
            "draft/message-redaction",
            &[],
            available,
            config,
        ) {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("account-notify", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) = self.create_request(
            "extended-join",
            &["account-notify"],
            available,
            config,
        ) {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("batch", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) = self.create_request(
            "draft/chathistory",
            &["batch"],
            available,
            config,
        ) {
            requested.push(request);
        }

        if let Some(request) = self.create_request(
            "draft/event-playback",
            &["batch", "draft/chathistory"],
            available,
            config,
        ) {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("labeled-response", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("echo-message", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("multi-prefix", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("draft/read-marker", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("setname", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) = self.create_request(
            "soju.im/bouncer-networks",
            &[],
            available,
            config,
        ) {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("sasl", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("draft/multiline", &[], available, config)
        {
            requested.push(request);
        }

        // TODO(quaff): remove `draft/no-implicit-names` support when ergo & soju have both been upgraded
        if let Some(request) =
            self.create_request("no-implicit-names", &[], available, config)
        {
            requested.push(request);
        } else if let Some(request) = self.create_request(
            "draft/no-implicit-names",
            &[],
            available,
            config,
        ) {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("draft/metadata-2", &[], available, config)
        {
            requested.push(request);
        }

        if let Some(request) =
            self.create_request("draft/whoami", &[], available, config)
        {
            requested.push(request);
        }

        requested
    }

    pub fn create_new_requested(
        &mut self,
        config: &config::Server,
    ) -> Vec<Cow<'static, str>> {
        let requested = self.create_requested(&self.pending, config);

        for (cap, val) in self.pending.drain() {
            self.listed.insert(cap, val);
        }

        requested
    }

    pub fn create_update_requested(
        &self,
        config: &config::Server,
    ) -> Vec<Cow<'static, str>> {
        self.create_requested(&self.listed, config)
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
    pub time: message::Time,
}

impl LabeledResponseContext {
    pub fn new(message: &message::Encoded, label: &str) -> Self {
        Self {
            // Prefix ':' to ensure it cannot match any valid message id
            label_as_id: format!(":label={label}").into(),
            time: message.time(),
        }
    }
}

// Server time from a message that indicates entry to a server or channel (i.e.
// RPL_ENDOFMOTD, ERR_NOMOTD, or JOIN);  if the message does not have a
// server_time tag, then the message::Time generated from local time will be
// shifted into the future.  If the resultant time is ahead of the server's
// clock, then deduplication should take care of any duplicate messages we
// receive.
pub fn chathistory_entry_server_time(
    message: message::Encoded,
) -> DateTime<Utc> {
    let time = message.time();

    if matches!(time.source, time::Source::Server) {
        time.utc
    } else {
        TimeDelta::try_minutes(90)
            .and_then(|time_delta| time.utc.checked_add_signed(time_delta))
            .unwrap_or(time.utc)
    }
}
