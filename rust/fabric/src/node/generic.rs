use crate::error::Result;
use crate::node::interface::{NodeConfig, NodeData, NodeInterface};
use async_trait::async_trait;

pub struct GenericNode {
    config: NodeConfig,
}

impl GenericNode {
    pub fn new(config: NodeConfig) -> Self {
        Self { config }
    }

    pub fn set_config(&mut self, config: NodeConfig) {
        self.config = config;
    }
}

#[async_trait]
impl NodeInterface for GenericNode {
    async fn read(&self) -> Result<f64> {
        // Implement generic read logic here
        Ok(0.0)
    }

    async fn read_data(&self) -> Result<NodeData> {
        // Implement generic read_data logic here
        Ok(NodeData {
            node_id: self.config.node_id.clone(),
            node_type: "generic".to_string(),
            status: "online".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
        })
    }

    fn get_config(&self) -> NodeConfig {
        self.config.clone()
    }

    fn set_config(&mut self, config: NodeConfig) {
        self.config = config;
    }

    fn get_type(&self) -> String {
        "generic".to_string()
    }

    async fn handle_event(&mut self, _event: &str, _payload: &str) -> Result<()> {
        // Implement generic event handling logic here
        Ok(())
    }

    fn update_config(&mut self, config: NodeConfig) {
        self.config = config;
    }
}
