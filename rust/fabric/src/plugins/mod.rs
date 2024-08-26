use crate::node::interface::{NodeConfig, NodeFactory, NodeInterface};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;

lazy_static! {
    static ref NODE_REGISTRY: RwLock<NodeRegistry> = RwLock::new(NodeRegistry::default());
}

#[derive(Default)]
pub struct NodeRegistry {
    factories: HashMap<String, Arc<dyn NodeFactory>>,
}

impl NodeRegistry {
    pub fn register<F: NodeFactory + 'static>(&mut self, node_type: &str, factory: F) {
        self.factories
            .insert(node_type.to_string(), Arc::new(factory));
    }

    pub fn create_node(
        &self,
        node_type: &str,
        config: NodeConfig,
    ) -> Option<Box<dyn NodeInterface>> {
        self.factories
            .get(node_type)
            .map(|factory| factory.create(config))
    }
}

pub fn register_node_type<F: NodeFactory + 'static>(node_type: &str, factory: F) {
    NODE_REGISTRY.write().unwrap().register(node_type, factory);
}

pub fn create_node(node_type: &str, config: NodeConfig) -> Option<Box<dyn NodeInterface>> {
    NODE_REGISTRY.read().unwrap().create_node(node_type, config)
}
