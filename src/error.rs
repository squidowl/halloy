use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    IcedError(#[from] iced::Error),

    #[error("{0}")]
    IoError(#[from] std::io::Error),
}
