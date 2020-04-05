use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("I/O error")]
    IoError(#[from] std::io::Error),
    #[error("Paese error")]
    ParseError(#[from] toml::ser::Error),
}
