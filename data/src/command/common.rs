#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Command {
    pub scope: Scope,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Scope {
    Network,
    #[default]
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub nick: String,
    pub channels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    UnknownOption(String),
    MissingScope,
    InvalidScope(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownOption(option) => {
                write!(f, "invalid common option: {option}")
            }
            Self::MissingScope => {
                write!(f, "invalid common syntax: expected scope value")
            }
            Self::InvalidScope(scope) => {
                write!(
                    f,
                    "invalid common scope: {scope}; expected network or global"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

/// Parses the `/common` option string.
///
/// Used by the top-level command parser. `raw` is the argument tail after
/// `/common`. Returns a typed command with `scope=global` as the default, or a
/// syntax error for unknown options or unsupported scope values. Produces no
/// output or side effects.
pub fn parse(raw: &str) -> Result<Command, Error> {
    let mut command = Command::default();

    for option in raw.split_whitespace() {
        let Some((key, value)) = option.split_once('=') else {
            return Err(Error::UnknownOption(option.to_string()));
        };

        if key != "scope" {
            return Err(Error::UnknownOption(option.to_string()));
        }

        command.scope = match value {
            "network" => Scope::Network,
            "global" => Scope::Global,
            "" => return Err(Error::MissingScope),
            _ => return Err(Error::InvalidScope(value.to_string())),
        };
    }

    Ok(command)
}

/// Normalizes `/common` result entries for display.
///
/// Used by buffer-side `/common` execution after membership scanning. `entries`
/// are candidate nick/channel rows. Returns entries with sorted/deduplicated
/// channel lists, users without shared channels removed, and rows sorted by
/// display nick. Produces no output or side effects.
pub fn summarize(entries: impl IntoIterator<Item = Entry>) -> Vec<Entry> {
    let mut entries: Vec<_> = entries
        .into_iter()
        .filter_map(|mut entry| {
            entry.channels.sort();
            entry.channels.dedup();

            (!entry.channels.is_empty()).then_some(entry)
        })
        .collect();

    entries.sort_by(|a, b| a.nick.cmp(&b.nick));
    entries
}
