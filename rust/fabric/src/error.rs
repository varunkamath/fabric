use thiserror::Error;

#[derive(Error, Debug)]
pub enum FabricError {
    #[error("Zenoh error: {0}")]
    ZenohError(#[from] zenoh::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, FabricError>;
