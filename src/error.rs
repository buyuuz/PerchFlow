use thiserror::Error;

#[derive(Error, Debug)]
pub enum PerchError {
    #[error("config error: {0}")]
    Config(String),
    #[error("process error: {0}")]
    Process(String),
    #[error("service error: {0}")]
    Service(String),
    #[error("web error: {0}")]
    Web(String),
}

pub type Result<T> = std::result::Result<T, PerchError>;
