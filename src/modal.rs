pub mod reload_configuration_error;

#[derive(Clone, Debug)]
pub enum Message {
    ReloadConfigurationError(reload_configuration_error::Message)
}

impl From<reload_configuration_error::Message> for Message {
    fn from(message: reload_configuration_error::Message) -> Self {
        Self::ReloadConfigurationError(message)
    }
}