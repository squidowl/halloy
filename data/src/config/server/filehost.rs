use serde::{Deserialize, Deserializer};

use super::Sasl;

#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Filehost {
    /// Whether to use the server's filehost. Defaults to `true`.
    pub enabled: bool,
    /// Override the filehost URL advertised by the server via ISUPPORT
    pub override_url: Option<String>,
    pub credentials: Credentials,
}

impl Default for Filehost {
    fn default() -> Self {
        Self {
            enabled: true,
            override_url: None,
            credentials: Credentials::default(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Default, Clone)]
pub enum Credentials {
    #[default]
    Server,
    Sasl(Sasl),
    None,
}

impl<'de> Deserialize<'de> for Credentials {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(untagged)]
        pub enum Data {
            String(String),
            Sasl(Sasl),
        }

        match Data::deserialize(deserializer)? {
            Data::String(string) => match string.as_str() {
                "server" => Ok(Credentials::Server),
                "none" => Ok(Credentials::None),
                _ => Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(&string),
                    &"valid non-SASL values are \"server\" and \"none\"",
                )),
            },
            Data::Sasl(sasl) => Ok(Credentials::Sasl(sasl)),
        }
    }
}
