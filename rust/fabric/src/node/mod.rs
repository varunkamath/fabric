#[allow(clippy::module_inception)]
mod node;
pub use node::Node;
pub mod generic;
pub mod interface;

use self::interface::NodeData;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeState {
    pub last_value: f64,
    pub last_update: std::time::SystemTime,
}

pub type CallbackFunction = Box<dyn Fn(NodeData) + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result; // Import our custom Result type
    use crate::node::interface::{NodeConfig, NodeFactory, NodeInterface};
    use std::sync::Arc;
    use zenoh::prelude::r#async::*;

    #[allow(dead_code)]
    struct MockNodeFactory;

    impl NodeFactory for MockNodeFactory {
        fn create(&self, config: NodeConfig) -> Box<dyn NodeInterface> {
            Box::new(MockNode { config })
        }
    }

    #[allow(dead_code)]
    struct MockNode {
        config: NodeConfig,
    }

    #[async_trait::async_trait]
    impl NodeInterface for MockNode {
        async fn read(&self) -> Result<f64> {
            Ok(42.0)
        }

        async fn read_data(&self) -> Result<NodeData> {
            Ok(NodeData {
                node_id: self.config.node_id.clone(),
                node_type: "mock".to_string(),
                value: 42.0,
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
            "mock".to_string()
        }

        async fn handle_event(&mut self, _event: &str, _payload: &str) -> Result<()> {
            Ok(())
        }

        fn update_config(&mut self, config: NodeConfig) {
            self.config = config;
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_node_new() {
        let config = zenoh::config::Config::default();
        let session = zenoh::open(config).res().await.unwrap();

        let node_config = NodeConfig {
            node_id: "test_node".to_string(),
            config: serde_json::json!({
                "sampling_rate": 5,
                "threshold": 50.0
            }),
        };

        let result = Node::new(
            "test_node".to_string(),
            "mock".to_string(),
            node_config,
            Arc::new(session),
            None, // Add this line to provide the config_updated_tx parameter
        )
        .await;

        assert!(result.is_ok(), "Node created successfully");
    }
}
