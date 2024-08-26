use std::error::Error as StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FabricError {
    #[error("Zenoh error: {0}")]
    ZenohError(#[from] zenoh::Error),
    #[error("Sensor error: {0}")]
    SensorError(String),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Other error: {0}")]
    Other(String),
    #[error(transparent)]
    StdError(Box<dyn StdError + Send + Sync>),
}

pub type Result<T> = std::result::Result<T, FabricError>;
