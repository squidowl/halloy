use std::ops;
use std::str::FromStr;

use iced_core::keyboard;
use serde::Deserialize;

pub fn shortcut(key_bind: KeyBind, command: Command) -> Shortcut {
    Shortcut { key_bind, command }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
                key_code: KeyCode(iced_core::keyboard::KeyCode::$k),
                modifiers: Modifiers::default(),
            }
        }
    };
    ($name:ident, $k:tt, $m:expr) => {
        pub fn $name() -> KeyBind {
            KeyBind {
                key_code: KeyCode(iced_core::keyboard::KeyCode::$k),
                modifiers: $m,
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct KeyBind {
    key_code: KeyCode,
    modifiers: Modifiers,
}

impl KeyBind {
    default!(move_up, Up, ALT);
    default!(move_down, Down, ALT);
    default!(move_left, Left, ALT);
    default!(move_right, Right, ALT);
    default!(close_buffer, W, COMMAND);
    default!(maximize_buffer, Up, COMMAND);
    default!(restore_buffer, Down, COMMAND);
    default!(cycle_next_buffer, Tab, CTRL);
    default!(cycle_previous_buffer, Tab, CTRL | SHIFT);
    default!(toggle_nick_list, M, COMMAND | ALT);

    pub fn is_pressed(
        &self,
        key_code: impl Into<KeyCode>,
        modifiers: impl Into<Modifiers>,
    ) -> bool {
        self.key_code == key_code.into() && self.modifiers == modifiers.into()
    }

    pub fn from_char(char: char, modifiers: impl Into<Modifiers>) -> Option<Self> {
        char.to_string()
            .parse::<KeyCode>()
            .ok()
            .map(|key_code| KeyBind {
                key_code,
                modifiers: modifiers.into(),
            })
    }
}

impl From<(keyboard::KeyCode, keyboard::Modifiers)> for KeyBind {
    fn from((key_code, modifiers): (keyboard::KeyCode, keyboard::Modifiers)) -> Self {
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

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub struct KeyCode(keyboard::KeyCode);

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
            "1" => keyboard::KeyCode::Key1,
            "2" => keyboard::KeyCode::Key2,
            "3" => keyboard::KeyCode::Key3,
            "4" => keyboard::KeyCode::Key4,
            "5" => keyboard::KeyCode::Key5,
            "6" => keyboard::KeyCode::Key6,
            "7" => keyboard::KeyCode::Key7,
            "8" => keyboard::KeyCode::Key8,
            "9" => keyboard::KeyCode::Key9,
            "0" => keyboard::KeyCode::Key0,
            "a" => keyboard::KeyCode::A,
            "b" => keyboard::KeyCode::B,
            "c" => keyboard::KeyCode::C,
            "d" => keyboard::KeyCode::D,
            "e" => keyboard::KeyCode::E,
            "f" => keyboard::KeyCode::F,
            "g" => keyboard::KeyCode::G,
            "h" => keyboard::KeyCode::H,
            "i" => keyboard::KeyCode::I,
            "j" => keyboard::KeyCode::J,
            "k" => keyboard::KeyCode::K,
            "l" => keyboard::KeyCode::L,
            "m" => keyboard::KeyCode::M,
            "n" => keyboard::KeyCode::N,
            "o" => keyboard::KeyCode::O,
            "p" => keyboard::KeyCode::P,
            "q" => keyboard::KeyCode::Q,
            "r" => keyboard::KeyCode::R,
            "s" => keyboard::KeyCode::S,
            "t" => keyboard::KeyCode::T,
            "u" => keyboard::KeyCode::U,
            "v" => keyboard::KeyCode::V,
            "w" => keyboard::KeyCode::W,
            "x" => keyboard::KeyCode::X,
            "y" => keyboard::KeyCode::Y,
            "z" => keyboard::KeyCode::Z,
            "escape" => keyboard::KeyCode::Escape,
            "f1" => keyboard::KeyCode::F1,
            "f2" => keyboard::KeyCode::F2,
            "f3" => keyboard::KeyCode::F3,
            "f4" => keyboard::KeyCode::F4,
            "f5" => keyboard::KeyCode::F5,
            "f6" => keyboard::KeyCode::F6,
            "f7" => keyboard::KeyCode::F7,
            "f8" => keyboard::KeyCode::F8,
            "f9" => keyboard::KeyCode::F9,
            "f10" => keyboard::KeyCode::F10,
            "f11" => keyboard::KeyCode::F11,
            "f12" => keyboard::KeyCode::F12,
            "f13" => keyboard::KeyCode::F13,
            "f14" => keyboard::KeyCode::F14,
            "f15" => keyboard::KeyCode::F15,
            "f16" => keyboard::KeyCode::F16,
            "f17" => keyboard::KeyCode::F17,
            "f18" => keyboard::KeyCode::F18,
            "f19" => keyboard::KeyCode::F19,
            "f20" => keyboard::KeyCode::F20,
            "f21" => keyboard::KeyCode::F21,
            "f22" => keyboard::KeyCode::F22,
            "f23" => keyboard::KeyCode::F23,
            "f24" => keyboard::KeyCode::F24,
            "snapshot" => keyboard::KeyCode::Snapshot,
            "scroll" => keyboard::KeyCode::Scroll,
            "pause" => keyboard::KeyCode::Pause,
            "insert" => keyboard::KeyCode::Insert,
            "home" => keyboard::KeyCode::Home,
            "delete" => keyboard::KeyCode::Delete,
            "end" => keyboard::KeyCode::End,
            "pagedown" => keyboard::KeyCode::PageDown,
            "pageup" => keyboard::KeyCode::PageUp,
            "left" => keyboard::KeyCode::Left,
            "up" => keyboard::KeyCode::Up,
            "right" => keyboard::KeyCode::Right,
            "down" => keyboard::KeyCode::Down,
            "backspace" => keyboard::KeyCode::Backspace,
            "enter" => keyboard::KeyCode::Enter,
            "space" => keyboard::KeyCode::Space,
            "compose" => keyboard::KeyCode::Compose,
            "caret" => keyboard::KeyCode::Caret,
            "numlock" => keyboard::KeyCode::Numlock,
            "numpad0" => keyboard::KeyCode::Numpad0,
            "numpad1" => keyboard::KeyCode::Numpad1,
            "numpad2" => keyboard::KeyCode::Numpad2,
            "numpad3" => keyboard::KeyCode::Numpad3,
            "numpad4" => keyboard::KeyCode::Numpad4,
            "numpad5" => keyboard::KeyCode::Numpad5,
            "numpad6" => keyboard::KeyCode::Numpad6,
            "numpad7" => keyboard::KeyCode::Numpad7,
            "numpad8" => keyboard::KeyCode::Numpad8,
            "numpad9" => keyboard::KeyCode::Numpad9,
            "numpadadd" => keyboard::KeyCode::NumpadAdd,
            "numpaddivide" => keyboard::KeyCode::NumpadDivide,
            "numpaddecimal" => keyboard::KeyCode::NumpadDecimal,
            "numpadcomma" => keyboard::KeyCode::NumpadComma,
            "numpadenter" => keyboard::KeyCode::NumpadEnter,
            "numpadequals" => keyboard::KeyCode::NumpadEquals,
            "numpadmultiply" => keyboard::KeyCode::NumpadMultiply,
            "numpadsubtract" => keyboard::KeyCode::NumpadSubtract,
            "abntc1" => keyboard::KeyCode::AbntC1,
            "abntc2" => keyboard::KeyCode::AbntC2,
            "apostrophe" => keyboard::KeyCode::Apostrophe,
            "apps" => keyboard::KeyCode::Apps,
            "asterisk" => keyboard::KeyCode::Asterisk,
            "at" => keyboard::KeyCode::At,
            "ax" => keyboard::KeyCode::Ax,
            "backslash" => keyboard::KeyCode::Backslash,
            "calculator" => keyboard::KeyCode::Calculator,
            "capital" => keyboard::KeyCode::Capital,
            "colon" => keyboard::KeyCode::Colon,
            "comma" => keyboard::KeyCode::Comma,
            "convert" => keyboard::KeyCode::Convert,
            "equals" => keyboard::KeyCode::Equals,
            "grave" => keyboard::KeyCode::Grave,
            "kana" => keyboard::KeyCode::Kana,
            "kanji" => keyboard::KeyCode::Kanji,
            "lalt" => keyboard::KeyCode::LAlt,
            "lbracket" => keyboard::KeyCode::LBracket,
            "lcontrol" => keyboard::KeyCode::LControl,
            "lshift" => keyboard::KeyCode::LShift,
            "lwin" => keyboard::KeyCode::LWin,
            "mail" => keyboard::KeyCode::Mail,
            "mediaselect" => keyboard::KeyCode::MediaSelect,
            "mediastop" => keyboard::KeyCode::MediaStop,
            "minus" => keyboard::KeyCode::Minus,
            "mute" => keyboard::KeyCode::Mute,
            "mycomputer" => keyboard::KeyCode::MyComputer,
            "navigateforward" => keyboard::KeyCode::NavigateForward, // also called "Next"
            "navigatebackward" => keyboard::KeyCode::NavigateBackward, // also called "Prior"
            "nexttrack" => keyboard::KeyCode::NextTrack,
            "noconvert" => keyboard::KeyCode::NoConvert,
            "oem102" => keyboard::KeyCode::OEM102,
            "period" => keyboard::KeyCode::Period,
            "playpause" => keyboard::KeyCode::PlayPause,
            "plus" => keyboard::KeyCode::Plus,
            "power" => keyboard::KeyCode::Power,
            "prevtrack" => keyboard::KeyCode::PrevTrack,
            "ralt" => keyboard::KeyCode::RAlt,
            "rbracket" => keyboard::KeyCode::RBracket,
            "rcontrol" => keyboard::KeyCode::RControl,
            "rshift" => keyboard::KeyCode::RShift,
            "rwin" => keyboard::KeyCode::RWin,
            "semicolon" => keyboard::KeyCode::Semicolon,
            "slash" => keyboard::KeyCode::Slash,
            "sleep" => keyboard::KeyCode::Sleep,
            "stop" => keyboard::KeyCode::Stop,
            "sysrq" => keyboard::KeyCode::Sysrq,
            "tab" => keyboard::KeyCode::Tab,
            "underline" => keyboard::KeyCode::Underline,
            "unlabeled" => keyboard::KeyCode::Unlabeled,
            "volumedown" => keyboard::KeyCode::VolumeDown,
            "volumeup" => keyboard::KeyCode::VolumeUp,
            "wake" => keyboard::KeyCode::Wake,
            "webback" => keyboard::KeyCode::WebBack,
            "webfavorites" => keyboard::KeyCode::WebFavorites,
            "webforward" => keyboard::KeyCode::WebForward,
            "webhome" => keyboard::KeyCode::WebHome,
            "webrefresh" => keyboard::KeyCode::WebRefresh,
            "websearch" => keyboard::KeyCode::WebSearch,
            "webstop" => keyboard::KeyCode::WebStop,
            "yen" => keyboard::KeyCode::Yen,
            "copy" => keyboard::KeyCode::Copy,
            "paste" => keyboard::KeyCode::Paste,
            "cut" => keyboard::KeyCode::Cut,
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
