use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait NodeInterface: Send + Sync {
    async fn read(&self) -> Result<f64>;
    fn get_config(&self) -> NodeConfig;
    fn set_config(&mut self, config: NodeConfig);
    fn get_type(&self) -> String;
    async fn handle_event(&mut self, event: &str, payload: &str) -> Result<()>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: String,
    pub sampling_rate: u64,
    pub threshold: f64,
    #[serde(flatten)]
    pub custom_config: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeData {
    pub node_id: String,
    pub node_type: String,
    pub value: f64,
    pub timestamp: u64,
    pub metadata: Option<serde_json::Value>,
}

pub trait NodeFactory: Send + Sync {
    fn create(&self, config: NodeConfig) -> Box<dyn NodeInterface>;
}
