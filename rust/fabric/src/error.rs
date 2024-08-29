use std::error::Error as StdError;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum FabricError {
    #[error("Zenoh error: {0}")]
    ZenohError(#[from] zenoh::Error),

    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Publisher not found for topic: {0}")]
    PublisherNotFound(String),

    #[error("Other error: {0}")]
    Other(String),

    #[error("Zenoh API error: {0}")]
    ZenohApiError(Box<dyn StdError + Send + Sync>),
}

impl From<JoinError> for FabricError {
    fn from(err: JoinError) -> Self {
        FabricError::Other(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, FabricError>;
