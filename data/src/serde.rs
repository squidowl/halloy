use std::path::{self, PathBuf};

use serde::{Deserialize, Deserializer};

pub fn deserialize_path_buf_with_tilde_expansion<'de, D>(
    deserializer: D,
) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let path_buf: PathBuf = Deserialize::deserialize(deserializer)?;

    Ok(tilde_expansion(path_buf))
}

pub fn deserialize_path_buf_with_tilde_expansion_maybe<'de, D>(
    deserializer: D,
) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    let path_buf: Option<PathBuf> = Deserialize::deserialize(deserializer)?;

    Ok(path_buf.map(tilde_expansion))
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

pub fn default_bool_true() -> bool {
    true
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
