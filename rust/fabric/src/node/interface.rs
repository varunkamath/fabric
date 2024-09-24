use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;
#[async_trait]
pub trait NodeInterface: Send + Sync {
    fn get_config(&self) -> NodeConfig;
    async fn set_config(&mut self, config: NodeConfig);
    fn get_type(&self) -> String;
    async fn handle_event(&mut self, event: &str, payload: &str) -> Result<()>;
    async fn update_config(&mut self, config: NodeConfig);
    fn as_any(&mut self) -> &mut dyn Any;
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeConfig {
    pub node_id: String,
    pub config: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeData {
    pub node_id: String,
    pub node_type: String,
    pub timestamp: u64,
    pub metadata: Option<serde_json::Value>,
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_status() -> String {
    "online".to_string()
}

impl NodeData {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            node_type: "".to_string(),
            timestamp: 0,
            metadata: None,
            status: default_status(),
        }
    }
    pub fn from_json(json: &str) -> Result<Self> {
        let node_data: NodeData = serde_json::from_str(json)?;
        Ok(node_data)
    }
    pub fn from_fields(
        node_id: String,
        node_type: String,
        timestamp: u64,
        metadata: Option<serde_json::Value>,
        status: String,
    ) -> Self {
        Self {
            node_id,
            node_type,
            timestamp,
            metadata,
            status,
        }
    }
    pub fn to_json(&self) -> Result<String> {
        let json = serde_json::to_string(self)?;
        Ok(json)
    }
    pub fn get(&self, key: &str) -> Result<String> {
        // Find key in metadata by turning metadata into a JSON object and then getting the value
        let metadata_json = serde_json::to_string(&self.metadata)?;
        let metadata_obj: serde_json::Value = serde_json::from_str(&metadata_json)?;
        let value = metadata_obj[key].to_string();
        Ok(value)
    }
    pub fn set_status(&mut self, status: String) -> Result<()> {
        self.status = status;
        Ok(())
    }
}

pub trait NodeFactory: Send + Sync {
    fn create(&self, config: NodeConfig) -> Box<dyn NodeInterface>;
}
