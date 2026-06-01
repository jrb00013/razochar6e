use thiserror::Error;

#[derive(Debug, Error)]
pub enum RazError {
    #[error("no supported battery charge backend on this system")]
    NoBackend,
    #[error("threshold out of range: {0}")]
    InvalidThreshold(String),
    #[error("backend {backend} failed: {message}")]
    Backend { backend: String, message: String },
    #[error("WSL bridge failed: {0}")]
    WslBridge(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type RazResult<T> = Result<T, RazError>;
