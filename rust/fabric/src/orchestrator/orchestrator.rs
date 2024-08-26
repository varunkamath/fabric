use super::{CallbackFunction, NodeState, OrchestratorConfig};
use crate::node::interface::{NodeConfig, NodeData};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

use crate::error::Result;

#[derive(Clone)]
pub struct Orchestrator {
    id: String,
    session: Arc<Session>,
    pub nodes: Arc<Mutex<HashMap<String, NodeState>>>,
    callbacks: Arc<Mutex<HashMap<String, CallbackFunction>>>,
}

impl Orchestrator {
    pub async fn new(id: String, session: Arc<Session>) -> Result<Self> {
        Ok(Self {
            id,
            session,
            nodes: Arc::new(Mutex::new(HashMap::new())),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn run(&self, cancel: CancellationToken) -> Result<()> {
        let subscriber = self.session.declare_subscriber("node/data").res().await?;

        while !cancel.is_cancelled() {
            tokio::select! {
                Ok(sample) = subscriber.recv_async() => {
                    if let Ok(payload) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                        if let Ok(data) = serde_json::from_str::<NodeData>(payload) {
                            println!("Orchestrator {} received data from node {}: {:.2}", self.id, data.node_id, data.value);
                            self.update_node_state(data.clone()).await;
                            self.trigger_callbacks(data).await;
                        }
                    }
                }
                _ = cancel.cancelled() => {
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn update_node_state(&self, data: NodeData) {
        let mut nodes = self.nodes.lock().await;
        nodes.insert(
            data.node_id.clone(),
            NodeState {
                last_value: data.value,
                last_update: std::time::SystemTime::now(),
            },
        );
    }

    async fn trigger_callbacks(&self, data: NodeData) {
        let callbacks = self.callbacks.lock().await;
        if let Some(callback) = callbacks.get(&data.node_id) {
            callback(data);
        }
    }

    pub async fn subscribe_to_node(
        &self,
        node_id: &str,
        callback: impl Fn(NodeData) + Send + Sync + 'static,
    ) -> Result<()> {
        let mut callbacks = self.callbacks.lock().await;
        callbacks.insert(node_id.to_string(), Box::new(callback));

        let subscriber = self.session.declare_subscriber("node/data").res().await?;

        tokio::spawn({
            let node_id = node_id.to_string();
            let callbacks = self.callbacks.clone();
            async move {
                while let Ok(sample) = subscriber.recv_async().await {
                    if let Ok(payload) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                        if let Ok(data) = serde_json::from_str::<NodeData>(payload) {
                            if data.node_id == node_id {
                                println!("Received data for node {}: {:?}", node_id, data);
                                let callbacks = callbacks.lock().await;
                                if let Some(callback) = callbacks.get(&node_id) {
                                    callback(data);
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn publish_node_config(&self, node_id: &str, config: &NodeConfig) -> Result<()> {
        let key = format!("node/{}/config", node_id);
        let config_json = serde_json::to_string(config)?;

        self.session.put(&key, config_json).res().await?;

        println!("Published configuration for node {}", node_id);
        Ok(())
    }

    pub async fn publish_node_configs(&self, config: &OrchestratorConfig) -> Result<()> {
        for node_config in &config.nodes {
            self.publish_node_config(&node_config.node_id, node_config)
                .await?;
        }
        Ok(())
    }
}
