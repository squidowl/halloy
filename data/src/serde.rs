use std::path::{self, PathBuf};

use chrono::format::StrftimeItems;
use serde::{Deserialize, Deserializer};

use crate::Config;

pub fn deserialize_path_buf_with_path_transformations<'de, D>(
    deserializer: D,
) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let path_buf: PathBuf = Deserialize::deserialize(deserializer)?;

    Ok(prefix_with_config_path(tilde_expansion(path_buf)))
}

pub fn deserialize_path_buf_with_path_transformations_maybe<'de, D>(
    deserializer: D,
) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    let path_buf: Option<PathBuf> = Deserialize::deserialize(deserializer)?;

    Ok(path_buf.map(tilde_expansion).map(prefix_with_config_path))
}

fn prefix_with_config_path(path_buf: PathBuf) -> PathBuf {
    if path_buf.is_relative() {
        Config::config_dir().join(path_buf)
    } else {
        path_buf
    }
}

fn tilde_expansion(path_buf: PathBuf) -> PathBuf {
    let mut expanded_path_buf = PathBuf::new();

    let mut components = path_buf.components();

    if let Some(first_component) = components.next() {
        match first_component {
            path::Component::Normal(os_str) if os_str == "~" => {
                if let Some(home_dir) = dirs_next::home_dir() {
                    expanded_path_buf.push(home_dir);
                } else {
                    expanded_path_buf.push(first_component);
                }
            }
            _ => {
                expanded_path_buf.push(first_component);
            }
        }
    }

    components.for_each(|component| expanded_path_buf.push(component));

    expanded_path_buf
}

pub fn fail_as_none<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    // We must fully consume valid json otherwise the error leaves the
    // deserializer in an invalid state and it'll still fail
    //
    // This assumes we always use a json format
    let intermediate = serde_json::Value::deserialize(deserializer)?;

    Ok(Option::<T>::deserialize(intermediate).unwrap_or_default())
}

pub fn deserialize_strftime_date<'de, D>(
    deserializer: D,
) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let strftime_string = String::deserialize(deserializer)?;

    if StrftimeItems::new(&strftime_string).parse().is_ok() {
        Ok(strftime_string)
    } else {
        Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Str(&strftime_string),
            &"valid strftime string",
        ))
    }
}

pub fn deserialize_strftime_date_maybe<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let strftime_string_maybe: Option<String> =
        Deserialize::deserialize(deserializer)?;

    if let Some(strftime_string) = &strftime_string_maybe
        && StrftimeItems::new(strftime_string).parse().is_err()
    {
        Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Str(strftime_string),
            &"valid strftime string",
        ))
    } else {
        Ok(strftime_string_maybe)
    }
}

pub fn deserialize_positive_integer_maybe<'de, D>(
    deserializer: D,
) -> Result<Option<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let integer_maybe: Option<u8> = Deserialize::deserialize(deserializer)?;

    if let Some(integer) = integer_maybe
        && integer == 0
    {
        Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(integer.into()),
            &"any positive integer",
        ))
    } else {
        Ok(integer_maybe)
    }
}

pub fn deserialize_positive_integer<'de, D, T>(
    deserializer: D,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + PartialEq + Default,
{
    let value: T = T::deserialize(deserializer)?;

    if value == T::default() {
        Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(0),
            &"any positive integer",
        ))
    } else {
        Ok(value)
    }
}

pub fn deserialize_empty_string_as_none<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok((!s.is_empty()).then_some(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_from_tilde_expansion() {
        let tests = [
            ("~", dirs_next::home_dir().expect("expected valid home dir")),
            (
                "~/.config/halloy/",
                dirs_next::home_dir()
                    .expect("expected valid home dir")
                    .join(".config/halloy"),
            ),
        ];
        for (tilde_str, directory) in tests {
            assert_eq!(tilde_expansion(tilde_str.into()), directory);
        }
    }
}
