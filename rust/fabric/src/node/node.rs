use crate::error::{FabricError, Result};
use crate::node::generic::GenericNode;
use crate::node::interface::NodeData;
use crate::node::interface::{NodeConfig, NodeInterface};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

struct Publisher {
    topic: String,
    zenoh_publisher: zenoh::publication::Publisher<'static>,
}

pub struct Subscriber {
    topic: String,
    callback: Arc<Mutex<dyn Fn(Sample) + Send + Sync>>,
    zenoh_subscriber: zenoh::subscriber::Subscriber<'static, ()>,
}

#[derive(Clone)]
pub struct Node {
    id: String,
    node_type: String,
    config: Arc<RwLock<NodeConfig>>,
    session: Arc<Session>,
    interface: Arc<Mutex<Box<dyn NodeInterface + Send + Sync>>>,
    publishers: Arc<RwLock<HashMap<String, Publisher>>>,
    subscribers: Arc<RwLock<HashMap<String, Subscriber>>>,
    subscriber_tx: mpsc::Sender<Sample>,
}

impl Node {
    pub async fn new(
        id: String,
        node_type: String,
        config: NodeConfig,
        session: Arc<Session>,
        interface: Option<Box<dyn NodeInterface + Send + Sync>>,
    ) -> Result<Self> {
        let (subscriber_tx, subscriber_rx) = mpsc::channel(100);
        let interface = match interface {
            Some(interface) => interface,
            None => Box::new(GenericNode::new(config.clone())),
        };

        let node = Node {
            id,
            node_type,
            config: Arc::new(RwLock::new(config)),
            session,
            interface: Arc::new(Mutex::new(interface)),
            publishers: Arc::new(RwLock::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            subscriber_tx,
        };

        // Spawn a task to handle subscriber samples
        let node_clone = node.clone();
        tokio::spawn(async move {
            node_clone.handle_subscriber_samples(subscriber_rx).await;
        });

        Ok(node)
    }

    pub async fn run(&self, cancel: CancellationToken) -> Result<()> {
        info!("Starting node {}", self.id);

        let key_expr = format!("node/{}/config", self.id);
        let config_subscriber = self
            .session
            .declare_subscriber(&key_expr)
            .res()
            .await
            .map_err(|e| FabricError::ZenohError(e))?;

        // Initial status update
        self.update_status("online".to_string()).await?;

        // Spawn a task for periodic status updates
        let status_update_task = {
            let cancel_clone = cancel.clone();
            let self_clone = self.clone();
            tokio::spawn(async move {
                let mut interval = interval(Duration::from_millis(1000));
                loop {
                    tokio::select! {
                        _ = cancel_clone.cancelled() => {
                            break;
                        }
                        _ = interval.tick() => {
                            if let Err(e) = self_clone.update_status("online".to_string()).await {
                                warn!("Failed to update status for node {}: {:?}", self_clone.id, e);
                            }
                        }
                    }
                }
            })
        };

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("Node {} received cancellation signal", self.id);
                    break;
                }
                sample = config_subscriber.recv_async() => {
                    match sample {
                        Ok(sample) => {
                            // TODO: Change this. Orchestrator publishes serialized JSON
                            let new_config: NodeConfig = serde_json::from_slice(sample.value.payload.contiguous().as_ref())
                                .map_err(|e| FabricError::SerdeJsonError(e))?;
                            info!("Node {} received new configuration: {:?}", self.id, new_config);
                            self.update_config(new_config).await?;
                        }
                        Err(e) => {
                            warn!("Error receiving configuration for node {}: {:?}", self.id, e);
                        }
                    }
                }
            }
        }

        // Wait for the status update task to complete
        status_update_task
            .await
            .map_err(|e| FabricError::Other(format!("Status update task error: {}", e)))?;

        info!("Node {} stopped", self.id);
        Ok(())
    }

    pub async fn update_config(&self, new_config: NodeConfig) -> Result<()> {
        self.interface
            .lock()
            .await
            .update_config(new_config.clone())
            .await;
        // Update the Node's config field
        let mut config = self.config.write().await;
        *config = new_config;
        Ok(())
    }

    pub async fn get_config(&self) -> NodeConfig {
        self.config.read().await.clone()
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_type(&self) -> &str {
        &self.node_type
    }

    pub async fn get_interface(&self) -> Result<Arc<Mutex<Box<dyn NodeInterface + Send + Sync>>>> {
        Ok(self.interface.clone())
    }

    pub async fn set_interface(
        &mut self,
        interface: Box<dyn NodeInterface + Send + Sync>,
    ) -> Result<()> {
        *self.interface.lock().await = interface;
        Ok(())
    }

    pub async fn update_status(&self, status: String) -> Result<()> {
        let node_data = NodeData {
            node_id: self.id.clone(),
            node_type: self.node_type.clone(),
            status,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| FabricError::Other(e.to_string()))?
                .as_secs(),
            metadata: None,
        };
        self.publish_node_status(&node_data).await
    }

    async fn publish_node_status(&self, node_data: &NodeData) -> Result<()> {
        let key_expr = format!("fabric/{}/status", self.id);
        let payload = serde_json::to_vec(node_data).map_err(|e| FabricError::SerdeJsonError(e))?;
        self.session
            .put(&key_expr, payload)
            .res()
            .await
            .map_err(|e| FabricError::ZenohError(e))?;
        debug!("Published status for node {}: {:?}", self.id, node_data);
        Ok(())
    }

    pub async fn create_publisher(&self, topic: String) -> Result<()> {
        let key_expr = topic.clone();
        let zenoh_publisher = self
            .session
            .declare_publisher(key_expr)
            .res()
            .await
            .map_err(|e| FabricError::ZenohError(e))?;

        let publisher = Publisher {
            topic: topic.clone(),
            zenoh_publisher,
        };
        debug!("Created publisher for topic: {}", publisher.topic.clone());

        self.publishers.write().await.insert(topic, publisher);
        Ok(())
    }

    pub async fn publish(&self, topic: &str, data: Vec<u8>) -> Result<()> {
        let publishers = self.publishers.read().await;
        if let Some(publisher) = publishers.get(topic) {
            publisher
                .zenoh_publisher
                .put(data)
                .res()
                .await
                .map_err(|e| FabricError::ZenohError(e))?;
            Ok(())
        } else {
            Err(FabricError::Other(format!(
                "Publisher not found for topic: {}",
                topic
            )))
        }
    }

    pub async fn create_subscriber(
        &self,
        topic: String,
        callback: Arc<Mutex<dyn Fn(Sample) + Send + Sync>>,
    ) -> Result<()> {
        let key_expr = topic.clone();
        let subscriber_tx = self.subscriber_tx.clone();
        let zenoh_subscriber = self
            .session
            .declare_subscriber(&key_expr)
            .callback(move |sample| {
                let tx = subscriber_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = tx.send(sample).await {
                        error!("Failed to send sample to handler: {:?}", e);
                    }
                });
            })
            .res()
            .await
            .map_err(|e| FabricError::ZenohError(e))?;

        let subscriber = Subscriber {
            topic: topic.clone(),
            callback,
            zenoh_subscriber,
        };

        debug!("Created subscriber for topic: {}", subscriber.topic);

        self.subscribers.write().await.insert(topic, subscriber);
        Ok(())
    }

    async fn handle_subscriber_samples(&self, mut rx: mpsc::Receiver<Sample>) {
        while let Some(sample) = rx.recv().await {
            let subscribers = self.subscribers.read().await;
            for subscriber in subscribers.values() {
                if subscriber
                    .zenoh_subscriber
                    .key_expr()
                    .intersects(sample.key_expr.as_keyexpr())
                {
                    let callback = subscriber.callback.lock().await;
                    callback(sample.clone());
                }
            }
        }
    }

    // Remove the old handle_subscriber_samples method
}
