use serde::{Deserialize, Deserializer};

/// The character encoding used to read from and write to a server.
///
/// Accepts any [WHATWG encoding label](https://encoding.spec.whatwg.org/#names-and-labels)
/// (e.g. `"utf-8"`, `"iso-8859-15"`), matched case-insensitively.
#[derive(Debug, Clone, Copy)]
pub struct Encoding(pub &'static encoding_rs::Encoding);

impl Default for Encoding {
    fn default() -> Self {
        Self(encoding_rs::UTF_8)
    }
}

impl PartialEq for Encoding {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Encoding {}

impl<'de> Deserialize<'de> for Encoding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let label = String::deserialize(deserializer)?;

        encoding_rs::Encoding::for_label(label.as_bytes())
            .map(Encoding)
            .ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "unrecognized character encoding `{label}`"
                ))
            })
    }
}
