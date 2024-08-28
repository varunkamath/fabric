use crate::error::{FabricError, Result};
use crate::node::generic::GenericNode;
use crate::node::interface::{NodeConfig, NodeData, NodeInterface}; // Add NodeData here
use flume::Receiver;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;
use zenoh::publication::Publisher;
use zenoh::subscriber::Subscriber;

pub struct Node {
    id: String,
    node_type: String,
    session: Arc<zenoh::Session>,
    interface: Arc<Mutex<Box<dyn NodeInterface>>>,
    publishers: Arc<Mutex<HashMap<String, Publisher<'static>>>>,
    subscribers: Arc<Mutex<HashMap<String, Subscriber<'static, Receiver<Sample>>>>>,
    callbacks: Arc<Mutex<HashMap<String, Arc<dyn Fn(Sample) + Send + Sync>>>>,
    config_updated: Arc<Notify>,
}

impl Node {
    pub async fn new(
        id: String,
        node_type: String,
        config: NodeConfig,
        session: Arc<zenoh::Session>,
        _config_updated_callback: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> Result<Self> {
        let config_updated = Arc::new(Notify::new());

        let node = Self {
            id: id.clone(),
            node_type: node_type.clone(),
            session: session.clone(),
            interface: Arc::new(Mutex::new(Box::new(GenericNode::new(config.clone())))),
            publishers: Arc::new(Mutex::new(HashMap::new())),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
            config_updated,
        };

        // Create the publisher during initialization
        node.declare_node_data_publisher().await?;

        Ok(node)
    }

    pub async fn declare_node_data_publisher(&self) -> Result<()> {
        let topic = format!("node/{}/data", self.id);
        let mut publishers = self.publishers.lock().await;
        if !publishers.contains_key("data") {
            let publisher = self.session.declare_publisher(topic.clone()).res().await?;
            publishers.insert("data".to_string(), publisher);
            info!("Node {} declared publisher for topic: {}", self.id, topic);
        }
        Ok(())
    }

    pub async fn run(&self, cancel: CancellationToken) -> Result<()> {
        info!("Node {} starting", self.id);

        info!("Node {} about to subscribe to config updates", self.id);
        self.subscribe_to_config_updates().await?;
        info!("Node {} subscribed to config updates", self.id);

        // Ensure the publisher exists before entering the loop
        let topic = format!("node/{}/data", self.id);
        if !self.publishers.lock().await.contains_key("data") {
            self.declare_node_data_publisher().await?;
        }

        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    let data = self.interface.lock().await.read_data().await?;
                    debug!("Node {} publishing data: {:?}", self.id, data);
                    let payload = serde_json::to_string(&data)?;
                    self.publish(&topic, payload).await?;
                }
                _ = cancel.cancelled() => {
                    info!("Node {} shutting down", self.id);
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn declare_publisher(&self, topic: String) -> Result<()> {
        let publisher = self.session.declare_publisher(topic.clone()).res().await?;
        self.publishers.lock().await.insert(topic, publisher);
        Ok(())
    }

    pub async fn publish(&self, topic: &str, payload: String) -> Result<()> {
        let publishers = self.publishers.lock().await;
        if let Some(publisher) = publishers.get("data") {
            publisher.put(payload).res().await?;
            Ok(())
        } else {
            Err(FabricError::PublisherNotFound(topic.to_string()))
        }
    }

    pub async fn subscribe(
        &self,
        topic: &str,
        callback: impl Fn(Sample) + Send + Sync + 'static,
    ) -> Result<()> {
        let subscriber = self.session.declare_subscriber(topic).res().await?;
        self.subscribers
            .lock()
            .await
            .insert(topic.to_string(), subscriber);
        self.callbacks
            .lock()
            .await
            .insert(topic.to_string(), Arc::new(callback));
        Ok(())
    }

    pub async fn unsubscribe(&self, topic: &str) -> Result<()> {
        if let Some(subscriber) = self.subscribers.lock().await.remove(topic) {
            subscriber.undeclare().res().await?;
        }
        self.callbacks.lock().await.remove(topic);
        Ok(())
    }

    pub fn get_node_type(&self) -> &str {
        &self.node_type
    }

    pub async fn get_config(&self) -> NodeConfig {
        self.interface.lock().await.get_config()
    }

    pub async fn publish_node_data(
        &self,
        value: f64,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        let node_data = NodeData {
            node_id: self.id.clone(),
            node_type: self.node_type.clone(),
            value,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata,
        };

        let topic = format!("node/{}/data", self.id);
        let payload = serde_json::to_string(&node_data)?;

        info!("Publishing node data: {:?} to topic: {}", node_data, topic);

        self.session
            .put(&topic, payload)
            .res()
            .await
            .map_err(FabricError::ZenohError)?;

        Ok(())
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub async fn subscribe_to_config_updates(&self) -> Result<()> {
        let config_topic = format!("node/{}/config", self.id);
        info!(
            "Node {} subscribing to config updates on topic: {}",
            self.id, config_topic
        );
        let interface = self.interface.clone();
        let node_id = self.id.clone();
        let config_updated = self.config_updated.clone();
        self.subscribe(&config_topic, move |sample| {
            let interface = interface.clone();
            let node_id = node_id.clone();
            let config_updated = config_updated.clone();
            tokio::spawn(async move {
                info!("Node {} received raw config update: {:?}", node_id, sample);
                match serde_json::from_slice::<NodeConfig>(&sample.value.payload.contiguous()) {
                    Ok(config) => {
                        info!(
                            "Node {} received valid config update: {:?}",
                            node_id, config
                        );
                        let mut interface = interface.lock().await;
                        interface.update_config(config.clone());
                        info!("Node {} updated config successfully", node_id);
                        info!("Node {} notifying config update", node_id);
                        config_updated.notify_one();
                    }
                    Err(e) => {
                        warn!("Node {} received invalid config update: {}", node_id, e);
                        warn!("Raw payload: {:?}", sample.value.payload);
                    }
                }
            });
        })
        .await?;
        info!("Node {} successfully subscribed to config updates", self.id);
        Ok(())
    }

    pub async fn update_config(&self, config: NodeConfig) {
        let mut interface = self.interface.lock().await;
        interface.update_config(config);
    }
}
