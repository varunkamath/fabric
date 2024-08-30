use crate::error::{FabricError, Result};
use crate::node::generic::GenericNode;
use crate::node::interface::NodeData;
use crate::node::interface::{NodeConfig, NodeInterface};
use log::{debug, info, warn};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[derive(Clone)]
pub struct Node {
    id: String,
    node_type: String,
    config: Arc<RwLock<NodeConfig>>,
    session: Arc<Session>,
    interface: Arc<Mutex<Box<dyn NodeInterface + Send + Sync>>>,
    data_notify: Arc<Notify>,
}

impl Node {
    pub async fn new(
        id: String,
        node_type: String,
        config: NodeConfig,
        session: Arc<Session>,
        interface: Option<Box<dyn NodeInterface + Send + Sync>>,
    ) -> Result<Self> {
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
            data_notify: Arc::new(Notify::new()),
        };

        Ok(node)
    }

    pub async fn run(&self, cancel: CancellationToken) -> Result<()> {
        info!("Starting node {}", self.id);

        let key_expr = format!("node/{}/config", self.id);
        let subscriber = self
            .session
            .declare_subscriber(&key_expr)
            .res()
            .await
            .map_err(|e| FabricError::ZenohError(e))?;

        let data_notify = self.data_notify.clone();
        let interface = self.interface.clone();
        let node_id = self.id.clone();

        // Initial status update
        self.update_status("online".to_string()).await?;

        // Spawn a task for periodic status updates
        let status_update_task = {
            let cancel_clone = cancel.clone();
            let self_clone = self.clone();
            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(1));
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

        tokio::spawn(async move {
            loop {
                let data = interface.lock().await.read_data().await;
                match data {
                    Ok(data) => {
                        debug!("Node {} read data: {:?}", node_id, data);
                        data_notify.notify_one();
                    }
                    Err(e) => {
                        warn!("Error reading data for node {}: {:?}", node_id, e);
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("Node {} received cancellation signal", self.id);
                    break;
                }
                sample = subscriber.recv_async() => {
                    match sample {
                        Ok(sample) => {
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
                _ = self.data_notify.notified() => {
                    let data = self.interface.lock().await.read_data().await?;
                    let key_expr = format!("node/{}/data", self.id);
                    let payload = serde_json::to_vec(&data).map_err(|e| FabricError::SerdeJsonError(e))?;
                    self.session.put(&key_expr, payload).res().await.map_err(|e| FabricError::ZenohError(e))?;
                    debug!("Published data for node {}: {:?}", self.id, data);
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
            .update_config(new_config.clone());
        // Update the Node's config field
        let mut config = self.config.write().await;
        *config = new_config;
        Ok(())
    }

    pub async fn get_config(&self) -> NodeConfig {
        self.config.read().await.clone()
    }

    pub async fn read(&self) -> Result<f64> {
        self.interface.lock().await.read().await
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_type(&self) -> &str {
        &self.node_type
    }

    pub async fn declare_node_data_publisher(&self) -> Result<()> {
        let key_expr = format!("node/{}/data", self.id);
        self.session
            .declare_publisher(key_expr)
            .res()
            .await
            .map_err(|e| FabricError::ZenohError(e))?;
        Ok(())
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

    async fn publish_node_data(&self, node_data: &NodeData) -> Result<()> {
        let key_expr = format!("node/{}/data", self.id);
        let payload = serde_json::to_vec(node_data).map_err(|e| FabricError::SerdeJsonError(e))?;
        self.session
            .put(&key_expr, payload)
            .res()
            .await
            .map_err(|e| FabricError::ZenohError(e))?;
        debug!("Published data for node {}: {:?}", self.id, node_data);
        Ok(())
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
}
