use serde::{Deserialize, Deserializer};

use super::Error;

const SERVICE: &str = "chat.halloy";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Password {
    #[default]
    Disabled,
    Enabled,
    Key(String),
}

impl Password {
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Password::Disabled)
    }

    pub fn key_or_default(
        &self,
        default: impl FnOnce() -> String,
    ) -> Option<String> {
        match self {
            Password::Disabled => None,
            Password::Enabled => Some(default()),
            Password::Key(key) => Some(key.clone()),
        }
    }
}

impl<'de> Deserialize<'de> for Password {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Data {
            Enabled(bool),
            Key(String),
        }

        match Data::deserialize(deserializer)? {
            Data::Enabled(true) => Ok(Password::Enabled),
            Data::Enabled(false) => Ok(Password::Disabled),
            Data::Key(key) => {
                if key.is_empty() {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(""),
                        &"non-empty keyring entry name",
                    ))
                } else {
                    Ok(Password::Key(key))
                }
            }
        }
    }
}

pub fn server_password_key(server: &str) -> String {
    format!("servers.{server}.password")
}

pub fn nick_password_key(server: &str) -> String {
    format!("servers.{server}.nick_password")
}

pub fn channel_key(server: &str, channel: &str) -> String {
    format!("servers.{server}.channel_keys.{channel}")
}

pub fn sasl_plain_password_key(server: &str) -> String {
    format!("servers.{server}.sasl.plain.password")
}

pub fn filehost_credentials_plain_password_key(server: &str) -> String {
    format!("servers.{server}.filehost.credentials.plain.password")
}

pub fn proxy_password_key(kind: &str) -> String {
    format!("proxy.{kind}.password")
}

pub fn server_proxy_password_key(server: &str, kind: &str) -> String {
    format!("servers.{server}.proxy.{kind}.password")
}

pub fn get_password(key: &str) -> Result<Option<String>, Error> {
    ensure_default_store(key)?;

    let entry = keyring_core::Entry::new(SERVICE, key).map_err(|error| {
        Error::Keyring {
            key: key.to_string(),
            error: error.to_string(),
        }
    })?;

    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(error) => Err(Error::Keyring {
            key: key.to_string(),
            error: error.to_string(),
        }),
    }
}

pub fn set_password(key: &str, password: &str) -> Result<(), Error> {
    ensure_default_store(key)?;

    let entry = keyring_core::Entry::new(SERVICE, key).map_err(|error| {
        Error::Keyring {
            key: key.to_string(),
            error: error.to_string(),
        }
    })?;

    entry
        .set_password(password)
        .map_err(|error| Error::Keyring {
            key: key.to_string(),
            error: error.to_string(),
        })
}

fn ensure_default_store(key: &str) -> Result<(), Error> {
    if keyring_core::get_default_store().is_some() {
        return Ok(());
    }

    set_platform_default_store().map_err(|error| Error::Keyring {
        key: key.to_string(),
        error: error.to_string(),
    })
}

#[cfg(target_os = "macos")]
fn set_platform_default_store() -> keyring_core::Result<()> {
    use apple_native_keyring_store::keychain::Store;

    keyring_core::set_default_store(Store::new()?);
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn set_platform_default_store() -> keyring_core::Result<()> {
    use zbus_secret_service_keyring_store::Store;

    keyring_core::set_default_store(Store::new()?);
    Ok(())
}

#[cfg(windows)]
fn set_platform_default_store() -> keyring_core::Result<()> {
    use windows_native_keyring_store::Store;

    keyring_core::set_default_store(Store::new()?);
    Ok(())
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    windows
)))]
fn set_platform_default_store() -> keyring_core::Result<()> {
    Err(keyring_core::Error::NotSupportedByStore(
        "No platform keyring store is configured for this target".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize)]
    struct PasswordFixture {
        password_keyring: Password,
    }

    #[test]
    fn password_deserializes_enabled_boolean() {
        let fixture: PasswordFixture =
            toml::from_str("password_keyring = true").unwrap();

        assert_eq!(fixture.password_keyring, Password::Enabled);
    }

    #[test]
    fn password_deserializes_custom_key() {
        let fixture: PasswordFixture =
            toml::from_str(r#"password_keyring = "custom.secret""#).unwrap();

        assert_eq!(
            fixture.password_keyring,
            Password::Key("custom.secret".to_string())
        );
    }

    #[test]
    fn password_rejects_empty_key() {
        let error =
            toml::from_str::<PasswordFixture>(r#"password_keyring = """#)
                .unwrap_err();

        assert!(
            error.to_string().contains("non-empty keyring entry name"),
            "{error}"
        );
    }

    #[test]
    fn password_deserializes_map_values() {
        #[derive(Debug, Deserialize)]
        struct Fixture {
            channel_keys_keyring: HashMap<String, Password>,
        }

        let fixture: Fixture =
            toml::from_str(r##"channel_keys_keyring = { "#halloy" = true }"##)
                .unwrap();

        assert_eq!(
            fixture.channel_keys_keyring.get("#halloy"),
            Some(&Password::Enabled)
        );
    }

    #[test]
    fn default_key_names_are_stable() {
        assert_eq!(server_password_key("libera"), "servers.libera.password");
        assert_eq!(nick_password_key("libera"), "servers.libera.nick_password");
        assert_eq!(
            channel_key("libera", "#halloy"),
            "servers.libera.channel_keys.#halloy"
        );
        assert_eq!(
            sasl_plain_password_key("libera"),
            "servers.libera.sasl.plain.password"
        );
        assert_eq!(
            filehost_credentials_plain_password_key("libera"),
            "servers.libera.filehost.credentials.plain.password"
        );
        assert_eq!(proxy_password_key("http"), "proxy.http.password");
        assert_eq!(
            server_proxy_password_key("libera", "socks5"),
            "servers.libera.proxy.socks5.password"
        );
    }
}
