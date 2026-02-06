use std::collections::HashSet;

use chrono::{DateTime, Local, Utc};
use iced::Color;
use serde::{Deserialize, Deserializer};

pub use self::channel::Channel;
pub use crate::appearance::theme::{alpha_color, alpha_color_calculate};
use crate::config::buffer::nickname::Nickname;
use crate::config::buffer::text_input::TextInput;
use crate::config::inclusivities::{Inclusivities, is_target_channel_included};
use crate::user::NickRef;
use crate::{Server, isupport, target};

pub mod channel;
pub mod nickname;
pub mod text_input;

use crate::buffer::{
    BacklogSeparator, DateSeparators, SkinTone, StatusMessagePrefix, Timestamp,
};
use crate::message::source;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Buffer {
    pub timestamp: Timestamp,
    pub nickname: Nickname,
    pub text_input: TextInput,
    pub channel: Channel,
    pub server_messages: ServerMessages,
    pub internal_messages: InternalMessages,
    pub status_message_prefix: StatusMessagePrefix,
    pub chathistory: ChatHistory,
    pub backlog_separator: BacklogSeparator,
    pub date_separators: DateSeparators,
    pub commands: Commands,
    pub emojis: Emojis,
    pub mark_as_read: MarkAsRead,
    pub url: Url,
    pub line_spacing: u32,
    pub scroll_position_on_open: ScrollPosition,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum NicknameClickAction {
    #[default]
    OpenQuery,
    InsertNickname,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Emojis {
    pub show_picker: bool,
    pub skin_tone: SkinTone,
    pub auto_replace: bool,
    pub characters_to_trigger_picker: usize,
}

impl Default for Emojis {
    fn default() -> Self {
        Self {
            show_picker: true,
            skin_tone: SkinTone::default(),
            auto_replace: true,
            characters_to_trigger_picker: 2,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Url {
    pub prompt_before_open: bool,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScrollPosition {
    #[default]
    OldestUnread,
    Newest,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MarkAsRead {
    pub on_application_exit: bool,
    pub on_buffer_close: OnBufferClose,
    pub on_scroll_to_bottom: bool,
    pub on_message_sent: bool,
}

impl Default for MarkAsRead {
    fn default() -> Self {
        Self {
            on_application_exit: false,
            on_buffer_close: OnBufferClose::default(),
            on_scroll_to_bottom: true,
            on_message_sent: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OnBufferClose {
    Bool(bool),
    Condition(OnBufferCloseCondition),
}

impl Default for OnBufferClose {
    fn default() -> Self {
        Self::Condition(OnBufferCloseCondition::ScrolledToBottom)
    }
}

impl OnBufferClose {
    pub fn mark_as_read(&self, is_scrolled_to_bottom: Option<bool>) -> bool {
        match self {
            OnBufferClose::Bool(mark) => *mark,
            OnBufferClose::Condition(_) => {
                is_scrolled_to_bottom.unwrap_or(false)
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OnBufferCloseCondition {
    ScrolledToBottom,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Quit {
    #[serde(
        deserialize_with = "crate::serde::deserialize_empty_string_as_none"
    )]
    pub default_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Part {
    #[serde(
        deserialize_with = "crate::serde::deserialize_empty_string_as_none"
    )]
    pub default_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SysInfo {
    pub cpu: bool,
    pub memory: bool,
    pub gpu: bool,
    pub os: bool,
    pub uptime: bool,
}

impl Default for SysInfo {
    fn default() -> Self {
        Self {
            cpu: true,
            memory: true,
            gpu: true,
            os: true,
            uptime: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Commands {
    pub show_description: bool,
    pub sysinfo: SysInfo,
    pub quit: Quit,
    pub part: Part,
}

impl Default for Commands {
    fn default() -> Self {
        Self {
            show_description: true,
            sysinfo: SysInfo::default(),
            quit: Quit::default(),
            part: Part::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Away {
    Dimmed(Dimmed),
    None,
}

impl Away {
    pub fn is_away(&self, is_user_away: bool) -> Option<Away> {
        is_user_away.then_some(*self)
    }
}

impl Default for Away {
    fn default() -> Self {
        Away::Dimmed(Dimmed::default())
    }
}

impl<'de> Deserialize<'de> for Away {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum AppearanceRepr {
            String(String),
            Struct(DimmedStruct),
        }

        #[derive(Deserialize)]
        struct DimmedStruct {
            dimmed: Option<f32>,
        }

        let repr = AppearanceRepr::deserialize(deserializer)?;
        match repr {
            AppearanceRepr::String(s) => match s.as_str() {
                "dimmed" => Ok(Away::Dimmed(Dimmed(None))),
                "solid" | "none" => Ok(Away::None),
                _ => Err(serde::de::Error::custom(format!(
                    "unknown appearance: {s}",
                ))),
            },
            AppearanceRepr::Struct(s) => Ok(Away::Dimmed(Dimmed(s.dimmed))),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ServerMessages {
    pub condense: Condensation,
    pub topic: ServerMessage,
    pub join: ServerMessage,
    pub part: ServerMessage,
    pub quit: ServerMessage,
    pub change_host: ServerMessage,
    pub change_mode: ServerMessage,
    pub change_nick: ServerMessage,
    pub monitored_online: ServerMessage,
    pub monitored_offline: ServerMessage,
    pub standard_reply_fail: ServerMessage,
    pub standard_reply_warn: ServerMessage,
    pub standard_reply_note: ServerMessage,
    pub wallops: ServerMessage,
    pub kick: ServerMessage,
    pub change_topic: ServerMessage,
    pub default: ServerMessageDefault,
}

impl ServerMessages {
    pub fn get(&self, kind: source::server::Kind) -> &ServerMessage {
        match kind {
            source::server::Kind::ReplyTopic => &self.topic,
            source::server::Kind::Join => &self.join,
            source::server::Kind::Part => &self.part,
            source::server::Kind::Quit => &self.quit,
            source::server::Kind::ChangeHost => &self.change_host,
            source::server::Kind::ChangeMode => &self.change_mode,
            source::server::Kind::ChangeNick => &self.change_nick,
            source::server::Kind::MonitoredOnline => &self.monitored_online,
            source::server::Kind::MonitoredOffline => &self.monitored_offline,
            source::server::Kind::StandardReply(
                source::server::StandardReply::Fail,
            ) => &self.standard_reply_fail,
            source::server::Kind::StandardReply(
                source::server::StandardReply::Warn,
            ) => &self.standard_reply_warn,
            source::server::Kind::StandardReply(
                source::server::StandardReply::Note,
            ) => &self.standard_reply_note,
            source::server::Kind::WAllOps => &self.wallops,
            source::server::Kind::Kick => &self.kick,
            source::server::Kind::ChangeTopic => &self.change_topic,
        }
    }

    pub fn dimmed(&self, server: Option<&source::Server>) -> Option<&Dimmed> {
        server.and_then(|server| {
            self.get(server.kind())
                .dimmed
                .as_ref()
                .or(self.default.dimmed.as_ref())
        })
    }

    pub fn enabled(&self, kind: source::server::Kind) -> bool {
        if let Some(enabled) = self.get(kind).enabled {
            enabled
        } else {
            self.default.enabled
        }
    }

    pub fn exclude(
        &self,
        kind: source::server::Kind,
    ) -> Option<&Inclusivities> {
        self.get(kind)
            .exclude
            .as_ref()
            .or(self.default.exclude.as_ref())
    }

    pub fn include(
        &self,
        kind: source::server::Kind,
    ) -> Option<&Inclusivities> {
        self.get(kind)
            .include
            .as_ref()
            .or(self.default.include.as_ref())
    }

    pub fn should_send_message(
        &self,
        kind: source::server::Kind,
        nick: Option<NickRef>,
        channel: &target::Channel,
        server: &Server,
        casemapping: isupport::CaseMap,
    ) -> bool {
        // Server Message is not enabled.
        if !self.enabled(kind) {
            return false;
        }

        is_target_channel_included(
            self.include(kind),
            self.exclude(kind),
            nick,
            channel,
            server,
            casemapping,
        )
    }

    pub fn smart(&self, kind: source::server::Kind) -> Option<i64> {
        self.get(kind).smart.or(self.default.smart)
    }

    pub fn username_format(
        &self,
        kind: source::server::Kind,
    ) -> UsernameFormat {
        self.get(kind)
            .username_format
            .unwrap_or(self.default.username_format)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Condensation {
    pub messages: HashSet<CondensationMessage>,
    pub format: CondensationFormat,
    #[serde(deserialize_with = "deserialize_dimmed_maybe")]
    pub dimmed: Option<Dimmed>,
}

impl Default for Condensation {
    fn default() -> Self {
        Self {
            messages: HashSet::new(),
            format: CondensationFormat::default(),
            dimmed: Some(Dimmed(None)),
        }
    }
}

impl Condensation {
    pub fn kind(&self, server: &source::Server) -> bool {
        match server.kind() {
            source::server::Kind::Join => {
                self.messages.contains(&CondensationMessage::Join)
            }
            source::server::Kind::Part => {
                self.messages.contains(&CondensationMessage::Part)
            }
            source::server::Kind::Quit => {
                self.messages.contains(&CondensationMessage::Quit)
            }
            source::server::Kind::ChangeNick => {
                self.messages.contains(&CondensationMessage::ChangeNick)
            }
            source::server::Kind::ChangeHost => {
                self.messages.contains(&CondensationMessage::ChangeHost)
            }
            source::server::Kind::Kick => {
                self.messages.contains(&CondensationMessage::Kick)
            }
            source::server::Kind::ReplyTopic
            | source::server::Kind::ChangeMode
            | source::server::Kind::MonitoredOnline
            | source::server::Kind::MonitoredOffline
            | source::server::Kind::StandardReply(_)
            | source::server::Kind::WAllOps
            | source::server::Kind::ChangeTopic => false,
        }
    }

    pub fn any(&self) -> bool {
        !self.messages.is_empty()
    }
}

#[derive(Debug, Clone, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum CondensationMessage {
    Join,
    Part,
    Quit,
    ChangeNick,
    ChangeHost,
    Kick,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CondensationFormat {
    #[default]
    Brief,
    Detailed,
    Full,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct ServerMessage {
    pub enabled: Option<bool>,
    pub smart: Option<i64>,
    pub username_format: Option<UsernameFormat>,
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
    #[serde(deserialize_with = "deserialize_dimmed_maybe")]
    pub dimmed: Option<Dimmed>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerMessageDefault {
    pub enabled: bool,
    pub smart: Option<i64>,
    pub username_format: UsernameFormat,
    pub exclude: Option<Inclusivities>,
    pub include: Option<Inclusivities>,
    #[serde(deserialize_with = "deserialize_dimmed_maybe")]
    pub dimmed: Option<Dimmed>,
}

impl Default for ServerMessageDefault {
    fn default() -> Self {
        Self {
            enabled: true,
            smart: None,
            username_format: UsernameFormat::default(),
            exclude: None,
            include: None,
            dimmed: Some(Dimmed(None)),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct InternalMessages {
    pub success: InternalMessage,
    pub error: InternalMessage,
}

impl InternalMessages {
    pub fn get(&self, server: &source::Status) -> Option<&InternalMessage> {
        match server {
            source::Status::Success => Some(&self.success),
            source::Status::Error => Some(&self.error),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct InternalMessage {
    pub enabled: bool,
    pub smart: Option<i64>,
}

impl Default for InternalMessage {
    fn default() -> Self {
        Self {
            enabled: true,
            smart: None,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LevelFilter {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ChatHistory {
    pub infinite_scroll: bool,
}

impl Default for ChatHistory {
    fn default() -> Self {
        Self {
            infinite_scroll: true,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UsernameFormat {
    Short,
    #[default]
    Full,
    #[serde(skip)]
    Mask,
}

impl Buffer {
    pub fn format_timestamp(
        &self,
        date_time: &DateTime<Utc>,
    ) -> Option<String> {
        if self.timestamp.format.is_empty() {
            return None;
        }

        Some(
            self.timestamp.brackets.format(
                date_time
                    .with_timezone(&Local)
                    .format(&self.timestamp.format),
            ),
        )
    }

    pub fn format_range_timestamp(
        &self,
        start_date_time: &DateTime<Utc>,
        end_date_time: &DateTime<Utc>,
    ) -> Option<(String, String, String)> {
        if self.timestamp.format.is_empty() {
            return None;
        }

        // Since timestamp ranges are used without a nickname or message marker,
        // do not include space delimiter in the format.
        Some((
            self.timestamp
                .brackets
                .format(format!(
                    "{}",
                    start_date_time
                        .with_timezone(&Local)
                        .format(&self.timestamp.format)
                ))
                .to_string(),
            " \u{2013} ".to_string(),
            self.timestamp
                .brackets
                .format(format!(
                    "{}",
                    end_date_time
                        .with_timezone(&Local)
                        .format(&self.timestamp.format)
                ))
                .to_string(),
        ))
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
pub struct Dimmed(Option<f32>);

impl Dimmed {
    pub fn new(alpha: Option<f32>) -> Self {
        Dimmed(alpha)
    }

    pub fn transform_color(&self, color: Color, background: Color) -> Color {
        match self.0 {
            // Calculate alpha based on background and foreground.
            None => alpha_color_calculate(0.20, 0.61, background, color),
            // Calculate alpha based on user defined alpha value.
            Some(a) => alpha_color(color, a),
        }
    }
}

pub fn deserialize_dimmed_maybe<'de, D>(
    deserializer: D,
) -> Result<Option<Dimmed>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Data {
        Boolean(bool),
        Float(f32),
    }

    let dimmed_maybe: Option<Data> = Deserialize::deserialize(deserializer)?;

    Ok(dimmed_maybe.and_then(|dimmed| match dimmed {
        Data::Boolean(dim) => dim.then_some(Dimmed(None)),
        Data::Float(dim) => Some(Dimmed(Some(dim))),
    }))
}
