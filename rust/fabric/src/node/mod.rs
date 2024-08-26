#[allow(clippy::module_inception)]
mod node;
pub use node::Node;
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

    struct MockNodeFactory;

    impl NodeFactory for MockNodeFactory {
        fn create(&self, config: NodeConfig) -> Box<dyn NodeInterface> {
            Box::new(MockNode { config })
        }
    }

    struct MockNode {
        config: NodeConfig,
    }

    #[async_trait::async_trait]
    impl NodeInterface for MockNode {
        async fn read(&self) -> Result<f64> {
            Ok(42.0)
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
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_node_new() {
        crate::plugins::register_node_type("mock", MockNodeFactory);

        let config = zenoh::config::Config::default();
        let session = zenoh::open(config).res().await.unwrap();

        let node_config = NodeConfig {
            node_id: "test_node".to_string(),
            sampling_rate: 5,
            threshold: 50.0,
            custom_config: serde_json::json!({}),
        };

        let result = Node::new(
            "test_node".to_string(),
            "mock".to_string(),
            node_config,
            Arc::new(session),
        )
        .await;

        match result {
            Ok(_) => assert!(true, "Node created successfully"),
            Err(e) => panic!("Failed to create node: {:?}", e),
        }
    }
}
