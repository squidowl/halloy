use std::ops;
use std::str::FromStr;

use iced_core::keyboard::{self, key};
use serde::Deserialize;

pub fn shortcut(key_bind: KeyBind, command: Command) -> Shortcut {
    Shortcut { key_bind, command }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    key_bind: KeyBind,
    command: Command,
}

impl Shortcut {
    pub fn execute(&self, key_bind: KeyBind) -> Option<Command> {
        (self.key_bind == key_bind).then_some(self.command)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    CloseBuffer,
    MaximizeBuffer,
    RestoreBuffer,
    CycleNextBuffer,
    CyclePreviousBuffer,
    ToggleNickList,
}

macro_rules! default {
    ($name:ident, $k:tt) => {
        pub fn $name() -> KeyBind {
            KeyBind {
                key_code: KeyCode($k),
                modifiers: Modifiers::default(),
            }
        }
    };
    ($name:ident, $k:tt, $m:expr) => {
        pub fn $name() -> KeyBind {
            KeyBind {
                key_code: KeyCode($k),
                modifiers: $m,
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct KeyBind {
    key_code: KeyCode,
    modifiers: Modifiers,
}

impl KeyBind {
    // default!(move_up, Up, ALT);
    // default!(move_down, Down, ALT);
    // default!(move_left, Left, ALT);
    // default!(move_right, Right, ALT);
    // default!(close_buffer, W, COMMAND);
    // default!(maximize_buffer, Up, COMMAND);
    // default!(restore_buffer, Down, COMMAND);
    // default!(cycle_next_buffer, Tab, CTRL);
    // default!(cycle_previous_buffer, Tab, CTRL | SHIFT);
    // default!(toggle_nick_list, M, COMMAND | ALT);

    pub fn is_pressed(
        &self,
        key_code: impl Into<KeyCode>,
        modifiers: impl Into<Modifiers>,
    ) -> bool {
        self.key_code == key_code.into() && self.modifiers == modifiers.into()
    }

    pub fn from_char(char: char, modifiers: impl Into<Modifiers>) -> Option<Self> {
        // char.to_string()
        //     .parse::<KeyCode>()
        //     .ok()
        //     .map(|key_code| KeyBind {
        //         key_code,
        //         modifiers: modifiers.into(),
        //     })
    }
}

impl From<(keyboard::Key, keyboard::Modifiers)> for KeyBind {
    fn from((key_code, modifiers): (keyboard::Key, keyboard::Modifiers)) -> Self {
        Self {
            key_code: KeyCode(key_code),
            modifiers: Modifiers(modifiers),
        }
    }
}

impl<'de> Deserialize<'de> for KeyBind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        let string = String::deserialize(deserializer)?;

        let parts = string.trim().split('+').collect::<Vec<_>>();

        let (key_code, modifiers) = match parts.len() {
            0 => return Err(de::Error::custom("empty keybind")),
            1 => (
                parts[0].parse::<KeyCode>().map_err(de::Error::custom)?,
                Modifiers::default(),
            ),
            _ => {
                let modifiers = parts[..parts.len() - 1]
                    .iter()
                    .map(|s| s.parse::<Modifiers>())
                    .collect::<Result<Vec<_>, ParseError>>()
                    .map_err(de::Error::custom)?
                    .into_iter()
                    .fold(Modifiers::default(), ops::BitOr::bitor);
                let key_code = parts[parts.len() - 1]
                    .parse::<KeyCode>()
                    .map_err(de::Error::custom)?;
                (key_code, modifiers)
            }
        };

        Ok(KeyBind {
            key_code,
            modifiers,
        })
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub struct KeyCode(keyboard::Key);

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Default)]
pub struct Modifiers(keyboard::Modifiers);

const CTRL: Modifiers = Modifiers(keyboard::Modifiers::CTRL);
const SHIFT: Modifiers = Modifiers(keyboard::Modifiers::SHIFT);
const ALT: Modifiers = Modifiers(keyboard::Modifiers::ALT);
const COMMAND: Modifiers = Modifiers(keyboard::Modifiers::COMMAND);

impl From<keyboard::Modifiers> for Modifiers {
    fn from(modifiers: keyboard::Modifiers) -> Self {
        Self(modifiers)
    }
}

impl ops::BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl FromStr for KeyCode {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(match s.to_ascii_lowercase().as_str() {
            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "0" | "a" | "b" | "c" | "d"
            | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n" | "o" | "p" | "q" | "r"
            | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" => keyboard::Key::Character(s.into()),
            "escape" | "esc" => keyboard::Key::Named(key::Named::Escape),
            "f1" => keyboard::Key::Named(key::Named::F1),
            "f2" => keyboard::Key::Named(key::Named::F2),
            "f3" => keyboard::Key::Named(key::Named::F3),
            "f4" => keyboard::Key::Named(key::Named::F4),
            "f5" => keyboard::Key::Named(key::Named::F5),
            "f6" => keyboard::Key::Named(key::Named::F6),
            "f7" => keyboard::Key::Named(key::Named::F7),
            "f8" => keyboard::Key::Named(key::Named::F8),
            "f9" => keyboard::Key::Named(key::Named::F9),
            "f10" => keyboard::Key::Named(key::Named::F10),
            "f11" => keyboard::Key::Named(key::Named::F11),
            "f12" => keyboard::Key::Named(key::Named::F12),
            "f13" => keyboard::Key::Named(key::Named::F13),
            "f14" => keyboard::Key::Named(key::Named::F14),
            "f15" => keyboard::Key::Named(key::Named::F15),
            "f16" => keyboard::Key::Named(key::Named::F16),
            "f17" => keyboard::Key::Named(key::Named::F17),
            "f18" => keyboard::Key::Named(key::Named::F18),
            "f19" => keyboard::Key::Named(key::Named::F19),
            "f20" => keyboard::Key::Named(key::Named::F20),
            "f21" => keyboard::Key::Named(key::Named::F21),
            "f22" => keyboard::Key::Named(key::Named::F22),
            "f23" => keyboard::Key::Named(key::Named::F23),
            "f24" => keyboard::Key::Named(key::Named::F24),
            "home" => keyboard::Key::Named(key::Named::Home),
            "delete" => keyboard::Key::Named(key::Named::Delete),
            "end" => keyboard::Key::Named(key::Named::End),
            "pagedown" => keyboard::Key::Named(key::Named::PageDown),
            "pageup" => keyboard::Key::Named(key::Named::PageUp),
            "left" => keyboard::Key::Named(key::Named::ArrowLeft),
            "up" => keyboard::Key::Named(key::Named::ArrowUp),
            "right" => keyboard::Key::Named(key::Named::ArrowRight),
            "down" => keyboard::Key::Named(key::Named::ArrowDown),
            "backspace" => keyboard::Key::Named(key::Named::Backspace),
            "enter" => keyboard::Key::Named(key::Named::Enter),
            "space" => keyboard::Key::Named(key::Named::Space),
            "numlock" => keyboard::Key::Named(key::Named::NumLock),
            "alt" => keyboard::Key::Named(key::Named::Alt),
            "tab" => keyboard::Key::Named(key::Named::Tab),
            "pause" => keyboard::Key::Named(key::Named::Pause),
            "insert" => keyboard::Key::Named(key::Named::Insert),
            "backspace" => keyboard::Key::Named(key::Named::Backspace),
            "delete" => keyboard::Key::Named(key::Named::Delete),
            "cut" => keyboard::Key::Named(key::Named::Cut),
            "paste" => keyboard::Key::Named(key::Named::Paste),
            "copy" => keyboard::Key::Named(key::Named::Copy),
            "volumedown" => keyboard::Key::Named(key::Named::AudioVolumeDown),
            "volumeup" => keyboard::Key::Named(key::Named::AudioVolumeUp),
            "shift" => keyboard::Key::Named(key::Named::Shift),
            "control" => keyboard::Key::Named(key::Named::Control),
            "mute" => keyboard::Key::Named(key::Named::AudioVolumeMute),
            "mediastop" => keyboard::Key::Named(key::Named::MediaStop),
            "mediapause" => keyboard::Key::Named(key::Named::MediaPause),
            "mediatracknext" => keyboard::Key::Named(key::Named::MediaTrackNext),
            "mediatrackprev" => keyboard::Key::Named(key::Named::MediaTrackPrevious),
            _ => return Err(ParseError::InvalidKeyCode(s.to_string())),
        }))
    }
}

impl FromStr for Modifiers {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(match s.to_lowercase().as_str() {
            "shift" => keyboard::Modifiers::SHIFT,
            "ctrl" => keyboard::Modifiers::CTRL,
            "alt" | "option" | "opt" => keyboard::Modifiers::ALT,
            "cmd" | "command" => keyboard::Modifiers::COMMAND,
            "logo" | "super" | "windows" => keyboard::Modifiers::LOGO,
            _ => return Err(ParseError::InvalidModifier(s.to_string())),
        }))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid keycode: {0}")]
    InvalidKeyCode(String),
    #[error("invalid modifier: {0}")]
    InvalidModifier(String),
}
