#[allow(clippy::module_inception)]
mod orchestrator;
pub use orchestrator::Orchestrator;

use crate::node::interface::{NodeConfig, NodeData};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeState {
    pub last_value: f64,
    pub last_update: std::time::SystemTime,
}

pub type CallbackFunction = Box<dyn Fn(NodeData) + Send + Sync>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub nodes: Vec<NodeConfig>,
}

#[cfg(test)]
mod tests {
    use crate::node::interface::NodeData;
    use crate::orchestrator::Orchestrator;
    use std::sync::Arc;
    use zenoh::prelude::r#async::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_orchestrator_new() {
        let config = zenoh::config::Config::default();
        let session = zenoh::open(config).res().await.unwrap();

        let orchestrator = Orchestrator::new("test_orchestrator".to_string(), Arc::new(session))
            .await
            .unwrap();

        assert_eq!(orchestrator.get_id(), "test_orchestrator");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_update_node_state() {
        let config = zenoh::config::Config::default();
        let session = zenoh::open(config).res().await.unwrap();

        let orchestrator = Orchestrator::new("test_orchestrator".to_string(), Arc::new(session))
            .await
            .unwrap();

        let node_data = NodeData {
            node_id: "test_node".to_string(),
            node_type: "radio".to_string(),
            value: 42.0,
            timestamp: 1234567890,
            metadata: None,
        };

        orchestrator.update_node_state(node_data.clone()).await;

        let nodes = orchestrator.nodes.lock().await;
        assert!(nodes.contains_key(&node_data.node_id));
        let state = nodes.get(&node_data.node_id).unwrap();
        assert_eq!(state.last_value, node_data.value);
    }
}
