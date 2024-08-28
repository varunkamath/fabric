use std::error::Error as StdError;
use thiserror::Error;

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

pub type Result<T> = std::result::Result<T, FabricError>;
