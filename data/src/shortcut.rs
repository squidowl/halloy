use std::hash::Hash;
use std::str::FromStr;
use std::{fmt, ops};

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
    pub fn execute(&self, key_bind: &KeyBind) -> Option<Command> {
        (self.key_bind == *key_bind).then_some(self.command)
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
    LeaveBuffer,
    ToggleNicklist,
    ToggleTopic,
    ToggleSidebar,
    ToggleFullscreen,
    CommandBar,
    ReloadConfiguration,
    FileTransfers,
    Logs,
    ThemeEditor,
    Highlights,
    QuitApplication,
    ScrollUpPage,
    ScrollDownPage,
    ScrollToTop,
    ScrollToBottom,
    CycleNextUnreadBuffer,
    CyclePreviousUnreadBuffer,
    MarkAsRead,
}

macro_rules! default {
    ($name:ident, $k:tt) => {
        pub fn $name() -> KeyBind {
            KeyBind {
                key_code: KeyCode(iced_core::keyboard::Key::Named(
                    iced_core::keyboard::key::Named::$k,
                )),
                modifiers: Modifiers::default(),
            }
        }
    };
    ($name:ident, $k:literal, $m:expr) => {
        pub fn $name() -> KeyBind {
            KeyBind {
                key_code: KeyCode(iced_core::keyboard::Key::Character(
                    $k.into(),
                )),
                modifiers: $m,
            }
        }
    };
    ($name:ident, $k:tt, $m:expr) => {
        pub fn $name() -> KeyBind {
            KeyBind {
                key_code: KeyCode(iced_core::keyboard::Key::Named(
                    iced_core::keyboard::key::Named::$k,
                )),
                modifiers: $m,
            }
        }
    };
}

#[derive(Debug, Clone, Eq, Ord, PartialOrd)]
pub struct KeyBind {
    key_code: KeyCode,
    modifiers: Modifiers,
}

impl fmt::Display for KeyBind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.modifiers, self.key_code)
    }
}

impl PartialEq for KeyBind {
    fn eq(&self, other: &Self) -> bool {
        if self.modifiers != other.modifiers {
            return false;
        }

        match (&self.key_code.0, &other.key_code.0) {
            // SHIFT modifier effects if this comes across as `a` or `A`, but
            // we explicitly define / check modifiers so it doesn't matter if
            // user defined it as `a` or `A` in their keymap
            (keyboard::Key::Character(a), keyboard::Key::Character(b)) => {
                a.to_lowercase() == b.to_lowercase()
            }
            (a, b) => a == b,
        }
    }
}

impl Hash for KeyBind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key_code.hash(state);
        self.modifiers.hash(state);
    }
}

// For defaults check the platform specific defaults:
// macOS: https://support.apple.com/en-us/102650
// Windows: https://support.microsoft.com/en-us/windows/keyboard-shortcuts-in-windows-dcc61a57-8ff0-cffe-9796-cb9706c75eec
// Linux FreeDesktop (not ready yet): https://wiki.freedesktop.org/www/Specifications/default-keys-spec/
// Linux - KDE: https://docs.kde.org/stable5/en/khelpcenter/fundamentals/kbd.html
// Linux - Gnome: https://help.gnome.org/users/gnome-help/stable/keyboard-nav.html.en

impl KeyBind {
    default!(move_up, ArrowUp, COMMAND | ALT);
    default!(move_down, ArrowDown, COMMAND | ALT);
    default!(move_left, ArrowLeft, COMMAND | ALT);
    default!(move_right, ArrowRight, COMMAND | ALT);
    default!(close_buffer, "w", COMMAND);
    default!(maximize_buffer, ArrowUp, COMMAND | SHIFT);
    default!(restore_buffer, ArrowDown, COMMAND | SHIFT);
    default!(cycle_next_buffer, Tab, CTRL);
    default!(cycle_previous_buffer, Tab, CTRL | SHIFT);
    default!(leave_buffer, "w", COMMAND | SHIFT);
    default!(toggle_nick_list, "m", COMMAND | ALT);
    default!(toggle_sidebar, "b", COMMAND | ALT);
    default!(toggle_topic, "t", COMMAND | ALT);
    #[cfg(target_os = "macos")]
    default!(toggle_fullscreen, "f", COMMAND | CTRL);
    #[cfg(not(target_os = "macos"))]
    default!(toggle_fullscreen, F11);
    default!(command_bar, "k", COMMAND);
    default!(reload_configuration, "r", COMMAND);
    default!(file_transfers, "j", COMMAND);
    default!(logs, "l", COMMAND);
    default!(theme_editor, "t", COMMAND);
    default!(highlights, "i", COMMAND);
    default!(scroll_up_page, PageUp);
    default!(scroll_down_page, PageDown);
    // Don't use HOME / END since text input is always focused
    default!(scroll_to_top, ArrowUp, COMMAND);
    default!(scroll_to_bottom, ArrowDown, COMMAND);
    default!(cycle_next_unread_buffer, "`", CTRL);
    default!(cycle_previous_unread_buffer, "`", CTRL | SHIFT);
    // Command + m is minimize in macOS
    default!(mark_as_read, "m", COMMAND | SHIFT);

    pub fn is_pressed(
        &self,
        key_code: impl Into<KeyCode>,
        modifiers: impl Into<Modifiers>,
    ) -> bool {
        self.key_code == key_code.into() && self.modifiers == modifiers.into()
    }
}

impl From<(keyboard::Key, keyboard::Modifiers)> for KeyBind {
    fn from(
        (key_code, modifiers): (keyboard::Key, keyboard::Modifiers),
    ) -> Self {
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

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut mods = vec![];
        let inner = self.0;

        if inner.contains(keyboard::Modifiers::SHIFT) {
            mods.push("Shift");
        }
        if inner.contains(keyboard::Modifiers::CTRL) {
            if cfg!(target_os = "macos") {
                // macOS: ⌃
                mods.push("\u{2303}");
            } else {
                mods.push("Ctrl");
            }
        }
        if inner.contains(keyboard::Modifiers::ALT) {
            if cfg!(target_os = "macos") {
                // macOS: ⌥
                mods.push("\u{2325}");
            } else {
                mods.push("Alt");
            }
        }
        if inner.contains(keyboard::Modifiers::LOGO) {
            // macOS: ⌘
            mods.push("\u{2318}");
        }

        if mods.is_empty() {
            write!(f, "")
        } else {
            write!(f, "{}", mods.join(" "))
        }
    }
}

impl fmt::Display for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = match self.0.clone() {
            key::Key::Named(name) => {
                let named = match name {
                    key::Named::F1 => "F1",
                    key::Named::F2 => "F2",
                    key::Named::F3 => "F3",
                    key::Named::F4 => "F4",
                    key::Named::F5 => "F5",
                    key::Named::F6 => "F6",
                    key::Named::F7 => "F7",
                    key::Named::F8 => "F8",
                    key::Named::F9 => "F9",
                    key::Named::F10 => "F10",
                    key::Named::F11 => "F11",
                    key::Named::F12 => "F12",
                    key::Named::F13 => "F13",
                    key::Named::F14 => "F14",
                    key::Named::F15 => "F15",
                    key::Named::F16 => "F16",
                    key::Named::F17 => "F17",
                    key::Named::F18 => "F18",
                    key::Named::F19 => "F19",
                    key::Named::F20 => "F20",
                    key::Named::F21 => "F21",
                    key::Named::F22 => "F22",
                    key::Named::F23 => "F23",
                    key::Named::F24 => "F24",
                    key::Named::Home => "Home",
                    key::Named::Delete => "Delete",
                    key::Named::End => "End",
                    key::Named::PageDown => "PageDown",
                    key::Named::PageUp => "PageUp",
                    key::Named::ArrowLeft => "←",
                    key::Named::ArrowUp => "↑",
                    key::Named::ArrowRight => "→",
                    key::Named::ArrowDown => "↓",
                    key::Named::Backspace => "Backspace",
                    key::Named::Enter => "Enter",
                    key::Named::Space => "Space",
                    key::Named::NumLock => "NumLock",
                    key::Named::Alt => "Alt",
                    key::Named::Tab => "Tab",
                    key::Named::Pause => "Pause",
                    key::Named::Insert => "Insert",
                    key::Named::Cut => "Cut",
                    key::Named::Paste => "Paste",
                    key::Named::Copy => "Copy",
                    key::Named::AudioVolumeDown => "VolumeDown",
                    key::Named::AudioVolumeUp => "VolumeUp",
                    key::Named::Shift => "Shift",
                    key::Named::Control => "Control",
                    key::Named::AudioVolumeMute => "Mute",
                    key::Named::MediaStop => "MediaStop",
                    key::Named::MediaPause => "MediaPause",
                    key::Named::MediaTrackNext => "MediaTrackNext",
                    key::Named::MediaTrackPrevious => "MediaTrackPrev",
                    _ => "",
                };

                named.to_string()
            }
            key::Key::Character(c) => c.to_uppercase(),
            key::Key::Unidentified => String::new(),
        };

        write!(f, "{key}")
    }
}

impl FromStr for KeyCode {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(match s.to_ascii_lowercase().as_str() {
            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "0" | "a"
            | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k"
            | "l" | "m" | "n" | "o" | "p" | "q" | "r" | "s" | "t" | "u"
            | "v" | "w" | "x" | "y" | "z" | "`" | "-" | "=" | "[" | "]"
            | "\\" | ";" | "'" | "," | "." | "/" => {
                keyboard::Key::Character(s.into())
            }
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
            "mediatracknext" => {
                keyboard::Key::Named(key::Named::MediaTrackNext)
            }
            "mediatrackprev" => {
                keyboard::Key::Named(key::Named::MediaTrackPrevious)
            }
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
