use std::collections::HashSet;
use std::str::FromStr;
use std::string::ToString;

use irc::proto::{self, Tags, command, format};

use crate::{Target, User, config, message};

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
    Multiline,
    MultiPrefix,
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
            "multi-prefix" => Ok(Self::MultiPrefix),
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
                MultilineBatchKind::PRIVMSG | MultilineBatchKind::ACTION => 7,
                MultilineBatchKind::NOTICE => 6,
            } + target.len()
                + relay_bytes
                + 6,
        )
    }
}

pub fn multiline_concat_lines(concat_bytes: usize, text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut last_line_start = 0;
    let mut prev_char_index = 0;

    for (char_index, _) in text.char_indices() {
        if char_index.saturating_sub(last_line_start) > concat_bytes {
            lines.push(&text[last_line_start..prev_char_index]);
            last_line_start = prev_char_index;
        }

        prev_char_index = char_index;
    }

    lines.push(&text[last_line_start..]);

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
            MultilineBatchKind::PRIVMSG | MultilineBatchKind::ACTION =>
                "PRIVMSG",
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
    ACTION,
    NOTICE,
}

#[derive(Debug, Default)]
pub struct Capabilities {
    listed: HashSet<String>,
    pending: HashSet<String>,
    acknowledged: HashSet<Capability>,
    multiline: Option<MultilineLimits>,
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

        if self.pending.contains("invite-notify")
            && !self.acknowledged(Capability::InviteNotify)
        {
            requested.push("invite-notify");
        }

        if self.pending.contains("userhost-in-names")
            && !self.acknowledged(Capability::UserhostInNames)
        {
            requested.push("userhost-in-names");
        }

        if self.pending.contains("away-notify")
            && !self.acknowledged(Capability::AwayNotify)
        {
            requested.push("away-notify");
        }

        if self.pending.contains("message-tags")
            && !self.acknowledged(Capability::MessageTags)
        {
            requested.push("message-tags");
        }

        if self.pending.contains("server-time")
            && !self.acknowledged(Capability::ServerTime)
        {
            requested.push("server-time");
        }

        if self.pending.contains("chghost")
            && !self.acknowledged(Capability::Chghost)
        {
            requested.push("chghost");
        }

        if self.pending.contains("extended-monitor")
            && !self.acknowledged(Capability::ExtendedMonitor)
        {
            requested.push("extended-monitor");
        }

        if self.pending.contains("account-notify")
            || self.acknowledged(Capability::AccountNotify)
        {
            if !self.acknowledged(Capability::AccountNotify) {
                requested.push("account-notify");
            }

            if self.pending.contains("extended-join")
                && !self.acknowledged(Capability::ExtendedJoin)
            {
                requested.push("extended-join");
            }
        }

        if self.pending.contains("batch")
            || self.acknowledged(Capability::Batch)
        {
            if !self.acknowledged(Capability::Batch) {
                requested.push("batch");
            }

            // We require batch for chathistory support
            if (self.pending.contains("draft/chathistory")
                && config.chathistory)
                || self.acknowledged(Capability::Chathistory)
            {
                if !self.acknowledged(Capability::Chathistory) {
                    requested.push("draft/chathistory");
                }

                if self.pending.contains("draft/event-playback")
                    && !self.acknowledged(Capability::EventPlayback)
                {
                    requested.push("draft/event-playback");
                }
            }
        }

        if self.pending.contains("labeled-response")
            && !self.acknowledged(Capability::LabeledResponse)
        {
            requested.push("labeled-response");
        }

        if self.pending.contains("echo-message")
            && !self.acknowledged(Capability::EchoMessage)
        {
            requested.push("echo-message");
        }

        if self.pending.contains("multi-prefix")
            && !self.acknowledged(Capability::MultiPrefix)
        {
            requested.push("multi-prefix");
        }

        if self.pending.contains("draft/read-marker")
            && !self.acknowledged(Capability::ReadMarker)
        {
            requested.push("draft/read-marker");
        }

        if self.pending.contains("setname")
            && !self.acknowledged(Capability::Setname)
        {
            requested.push("setname");
        }

        if self.pending.contains("soju.im/bouncer-networks")
            && !self.acknowledged(Capability::BouncerNetworks)
        {
            requested.push("soju.im/bouncer-networks");
        }

        if self.pending.iter().any(|cap| cap.starts_with("sasl"))
            && !self.acknowledged(Capability::Sasl)
        {
            requested.push("sasl");
        }

        if let Some(multiline) = self
            .pending
            .iter()
            .find_map(|cap| cap.strip_prefix("draft/multiline="))
        {
            let dictionary = multiline.split(',').collect::<Vec<_>>();

            if let Some(max_bytes) = dictionary.iter().find_map(|key_value| {
                key_value
                    .strip_prefix("max-bytes=")
                    .and_then(|value| value.parse::<usize>().ok())
            }) {
                self.multiline = Some(MultilineLimits {
                    max_bytes,
                    max_lines: dictionary.iter().find_map(|key_value| {
                        key_value
                            .strip_prefix("max-lines=")
                            .and_then(|value| value.parse::<usize>().ok())
                    }),
                });

                if !self.acknowledged(Capability::Multiline) {
                    requested.push("draft/multiline");
                }
            }
        }

        for cap in self.pending.drain() {
            self.listed.insert(cap);
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

    pub fn extend_list(&mut self, caps: impl Iterator<Item = String>) {
        for cap in caps {
            self.pending.insert(cap);
        }
    }

    pub fn multiline_limits(&self) -> Option<MultilineLimits> {
        if self.acknowledged(Capability::Multiline) {
            self.multiline
        } else {
            None
        }
    }
}
