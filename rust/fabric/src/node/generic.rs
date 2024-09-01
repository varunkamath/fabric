use crate::error::Result;
use crate::node::interface::{NodeConfig, NodeInterface};
use async_trait::async_trait;
use std::any::Any;

pub struct GenericNode {
    config: NodeConfig,
}

impl GenericNode {
    pub fn new(config: NodeConfig) -> Self {
        Self { config }
    }

    pub async fn set_config(&mut self, config: NodeConfig) {
        self.config = config;
    }
}

#[async_trait]
impl NodeInterface for GenericNode {
    fn get_config(&self) -> NodeConfig {
        self.config.clone()
    }

    async fn set_config(&mut self, config: NodeConfig) {
        self.config = config;
    }

    fn get_type(&self) -> String {
        "generic".to_string()
    }

    async fn handle_event(&mut self, _event: &str, _payload: &str) -> Result<()> {
        // Implement generic event handling logic here
        Ok(())
    }

    async fn update_config(&mut self, config: NodeConfig) {
        self.config = config;
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}
