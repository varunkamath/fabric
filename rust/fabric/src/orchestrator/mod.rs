#[allow(clippy::module_inception)]
mod orchestrator;
pub use orchestrator::Orchestrator;

use crate::node::interface::{NodeConfig, NodeData};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct NodeState {
    pub last_value: crate::node::interface::NodeData,
    pub last_update: std::time::SystemTime,
}

impl NodeState {
    pub fn new(node_data: crate::node::interface::NodeData) -> Self {
        Self {
            last_value: node_data,
            last_update: std::time::SystemTime::now(),
        }
    }
}

pub type CallbackFunction = Box<dyn Fn(NodeData) + Send + Sync>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub nodes: Vec<NodeConfig>,
}

// Move the Orchestrator implementation here (if it's not already in the orchestrator.rs file)
impl Orchestrator {
    // ... (Orchestrator implementation)
}

// Move the tests module to the end of the file
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::interface::NodeData;

    #[test]
    fn test_node_state_new() {
        let node_data = NodeData {
            node_id: "test_node".to_string(),
            node_type: "test_type".to_string(),
            status: "online".to_string(),
            timestamp: 1234567890,
            metadata: None,
        };

        let node_state = NodeState::new(node_data.clone());

        assert_eq!(node_state.last_value, node_data);
        assert!(node_state.last_update <= std::time::SystemTime::now());
    }
}
