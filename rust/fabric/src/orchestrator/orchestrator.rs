use super::{CallbackFunction, NodeState};
use crate::node::interface::{NodeConfig, NodeData};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;
use zenoh::subscriber::Subscriber;

use crate::error::Result;

#[derive(Clone)]
pub struct Orchestrator {
    id: String,
    session: Arc<Session>,
    pub nodes: Arc<Mutex<HashMap<String, NodeState>>>,
    callbacks: Arc<Mutex<HashMap<String, CallbackFunction>>>,
    subscribers: Arc<Mutex<HashMap<String, Subscriber<'static, ()>>>>,
}

impl Orchestrator {
    pub async fn new(id: String, session: Arc<Session>) -> Result<Self> {
        info!("Creating new orchestrator: {}", id);
        Ok(Self {
            id,
            session,
            nodes: Arc::new(Mutex::new(HashMap::new())),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn run(&self, cancel: CancellationToken) -> Result<()> {
        info!("Orchestrator {} starting", self.id);
        let subscriber = self.session.declare_subscriber("node/*/data").res().await?;

        loop {
            tokio::select! {
                Ok(sample) = subscriber.recv_async() => {
                    debug!("Orchestrator {} received raw data: {:?}", self.id, sample);
                    if let Ok(node_data) = serde_json::from_slice::<NodeData>(&sample.value.payload.contiguous()) {
                        info!("Orchestrator {} parsed node data: {:?}", self.id, node_data);
                        self.update_node_state(node_data).await;
                    } else {
                        warn!("Failed to parse node data");
                    }
                }
                _ = cancel.cancelled() => {
                    info!("Orchestrator {} shutting down", self.id);
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn subscribe_to_node(&self, node_id: &str, callback: CallbackFunction) -> Result<()> {
        let topic = format!("node/{}/data", node_id);
        info!("Orchestrator subscribing to topic: {}", topic);
        let topic_clone = topic.clone(); // Clone the topic here
        let subscriber = self
            .session
            .declare_subscriber(&topic)
            .callback(move |sample| {
                if let Ok(node_data) =
                    serde_json::from_slice::<NodeData>(&sample.value.payload.contiguous())
                {
                    info!(
                        "Orchestrator received data on topic {}: {:?}",
                        topic_clone, node_data
                    );
                    callback(node_data);
                } else {
                    warn!("Failed to parse node data from topic: {}", topic_clone);
                }
            })
            .res()
            .await?;

        self.subscribers
            .lock()
            .await
            .insert(node_id.to_string(), subscriber);

        Ok(())
    }

    pub async fn publish_node_config(&self, node_id: &str, config: &NodeConfig) -> Result<()> {
        let key = format!("node/{}/config", node_id);
        let config_json = serde_json::to_string(config)?;
        info!("Orchestrator {} publishing config to key: {}", self.id, key);
        info!("Config payload: {}", config_json);
        self.session.put(&key, config_json).res().await?;
        info!(
            "Orchestrator {} successfully published config to node {}: {:?}",
            self.id, node_id, config
        );
        Ok(())
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub async fn update_node_state(&self, node_data: NodeData) {
        let mut nodes = self.nodes.lock().await;
        nodes.insert(
            node_data.node_id.clone(),
            NodeState {
                last_value: node_data.value,
                last_update: std::time::SystemTime::now(),
            },
        );

        let callbacks = self.callbacks.lock().await;
        if let Some(callback) = callbacks.get(&node_data.node_id) {
            callback(node_data);
        }
    }
}
