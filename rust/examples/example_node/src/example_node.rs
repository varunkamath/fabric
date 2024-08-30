use fabric::error::{FabricError, Result};
use fabric::node::interface::NodeConfig;
use fabric::node::Node;
use fabric::orchestrator::Orchestrator;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

pub struct ExampleNode {
    pub config: NodeConfig,
}

#[async_trait]
impl NodeInterface for ExampleNode {
    async fn read_data(&self) -> Result<NodeData> {
        Ok(NodeData {
            node_id: self.config.node_id.clone(),
            node_type: "example".to_string(),
            value: rand::random::<f64>(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: Some(serde_json::json!({
                    "example_data": "Some value",
                    "timestamp": chrono::Utc::now().timestamp(),
            })),
        })
    }

    fn get_config(&self) -> NodeConfig {
        self.config.clone()
    }

    fn update_config(&mut self, new_config: NodeConfig) {
        self.config = new_config;
    }

    async fn read(&self) -> Result<f64> {
        let config = self.config.clone();
        let value = rand::random::<f64>() * config.config["multiplier"].as_f64().unwrap_or(1.0);
        Ok(value)
    }

    fn set_config(&mut self, config: NodeConfig) {
        self.config = config;
    }

    fn get_type(&self) -> String {
        "example".to_string()
    }

    async fn handle_event(&mut self, topic: &str, payload: &str) -> Result<()> {
        println!(
            "Received event on topic '{}' with payload: {}",
            topic, payload
        );
        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        Ok(())
    }
}
