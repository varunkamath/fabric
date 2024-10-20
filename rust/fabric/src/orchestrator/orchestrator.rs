use super::NodeState;
use crate::error::{FabricError, Result};
use crate::node::interface::{NodeConfig, NodeData};
use backoff::{backoff::Backoff, ExponentialBackoff};
use log::{debug, error, info, warn};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

// Add this near the top of the file, after the imports
type NodeDataCallback = Arc<Mutex<dyn Fn(NodeData) + Send + Sync>>;

pub struct Publisher {
    topic: String,
    zenoh_publisher: zenoh::publication::Publisher<'static>,
}

pub struct Subscriber {
    topic: String,
    callback: Arc<Mutex<dyn Fn(Sample) + Send + Sync>>,
    zenoh_subscriber: zenoh::subscriber::Subscriber<'static, ()>,
}

#[derive(Clone)]
pub struct Orchestrator {
    id: String,
    pub session: Arc<Session>,
    pub nodes: Arc<Mutex<HashMap<String, NodeState>>>,
    callbacks: Arc<Mutex<HashMap<String, NodeDataCallback>>>,
    pub subscribers: Arc<RwLock<HashMap<String, Subscriber>>>,
    pub publishers: Arc<RwLock<HashMap<String, Publisher>>>,
    status_subscriber: Arc<Mutex<Option<zenoh::subscriber::Subscriber<'static, ()>>>>,
    subscriber_tx: mpsc::Sender<Sample>,
}

impl Orchestrator {
    pub async fn new(id: String, session: Arc<Session>) -> Result<Arc<Self>> {
        info!("Creating new orchestrator: {}", id);
        let (subscriber_tx, subscriber_rx) = mpsc::channel(100);
        let orchestrator = Self {
            id,
            session,
            nodes: Arc::new(Mutex::new(HashMap::new())),
            callbacks: Arc::new(Mutex::new(HashMap::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            publishers: Arc::new(RwLock::new(HashMap::new())),
            status_subscriber: Arc::new(Mutex::new(None)),
            subscriber_tx,
        };

        // Spawn a task to handle subscriber samples
        let orchestrator_clone = orchestrator.clone();
        tokio::spawn(async move {
            orchestrator_clone
                .handle_subscriber_samples(subscriber_rx)
                .await;
        });

        Ok(Arc::new(orchestrator))
    }

    pub async fn run(&self, cancel: CancellationToken) -> Result<()> {
        info!("Starting orchestrator: {}", self.id);

        // Subscribe to all node status topics
        self.subscribe_to_node_statuses().await?;

        // Start a task to check for offline nodes
        let offline_check_task = {
            let self_clone = self.clone();
            let cancel_clone = cancel.clone();
            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(1));
                loop {
                    tokio::select! {
                        _ = cancel_clone.cancelled() => break,
                        _ = interval.tick() => self_clone.check_offline_nodes().await,
                    }
                }
            })
        };

        // Wait for cancellation
        cancel.cancelled().await;
        info!("Orchestrator {} shutting down", self.id);

        // Unsubscribe from node status topics
        self.unsubscribe_from_node_statuses().await?;

        // Wait for the offline check task to complete
        offline_check_task
            .await
            .map_err(|e| FabricError::Other(format!("Offline check task error: {}", e)))?;

        info!("Orchestrator {} shutdown complete", self.id);

        Ok(())
    }

    pub async fn subscribe_to_node_statuses(&self) -> Result<()> {
        let orchestrator = self.clone();
        let subscriber = self
            .session
            .declare_subscriber("fabric/*/status")
            .callback(move |sample| {
                let orchestrator_clone = orchestrator.clone();
                tokio::spawn(async move {
                    orchestrator_clone.update_node_health(sample).await;
                });
            })
            .res()
            .await
            .map_err(FabricError::ZenohError)?;

        let mut status_subscriber = self.status_subscriber.lock().await;
        *status_subscriber = Some(subscriber);

        Ok(())
    }

    pub async fn unsubscribe_from_node_statuses(&self) -> Result<()> {
        info!("Unsubscribing from node statuses");
        let mut status_subscriber = self.status_subscriber.lock().await;
        if let Some(subscriber) = status_subscriber.take() {
            subscriber
                .undeclare()
                .res()
                .await
                .map_err(FabricError::ZenohError)?;
        }
        Ok(())
    }

    async fn update_node_health(&self, sample: Sample) {
        let key_expr = sample.key_expr.as_str();
        let node_id = key_expr.split('/').nth(1).unwrap_or("unknown");
        info!("Received health update for node: {}", node_id);

        // Convert ZBuf to a contiguous slice of bytes
        let payload_bytes = sample.value.payload.contiguous();

        // Deserialize the payload into a serde_json::Value
        match serde_json::from_slice::<serde_json::Value>(&payload_bytes) {
            Ok(json_value) => {
                debug!("Deserialized JSON: {:?}", json_value);

                let mut nodes = self.nodes.lock().await;
                let node_state = nodes
                    .entry(node_id.to_string())
                    .or_insert_with(|| NodeState {
                        last_value: NodeData::from_json(&json_value.to_string()).unwrap(),
                        last_update: std::time::SystemTime::now(),
                    });

                if let Ok(node_data) = NodeData::from_json(&json_value.to_string()) {
                    node_state.last_value = node_data;
                    node_state.last_update = std::time::SystemTime::now();

                    if node_state.last_value.status != "online" {
                        warn!("Node {} is {}", node_id, node_state.last_value.status);
                    }

                    // Trigger callbacks
                    let callbacks = self.callbacks.lock().await;
                    if let Some(callback) = callbacks.get(node_id) {
                        let callback = callback.lock().await;
                        callback(node_state.last_value.clone());
                    }
                } else {
                    warn!("Failed to parse NodeData from JSON for node {}", node_id);
                }
            }
            Err(e) => {
                warn!("Failed to parse JSON payload for node {}: {}", node_id, e);
            }
        }
    }

    pub async fn publish_node_config(&self, node_id: &str, config: &NodeConfig) -> Result<()> {
        let key = format!("node/{}/config", node_id);
        let config_json = serde_json::to_string(config)?;
        let mut backoff = ExponentialBackoff::default();

        loop {
            match self.session.put(&key, config_json.clone()).res().await {
                Ok(_) => {
                    info!(
                        "Orchestrator {} successfully published config to node {}: {:?}",
                        self.id, node_id, config
                    );
                    return Ok(());
                }
                Err(err) => {
                    if let Some(duration) = backoff.next_backoff() {
                        warn!(
                            "Failed to publish config, retrying in {:?}: {}",
                            duration, err
                        );
                        sleep(duration).await;
                    } else {
                        return Err(FabricError::PublishError(err.to_string()));
                    }
                }
            }
        }
    }

    pub async fn update_node_state(&self, node_data: NodeData) {
        let mut nodes = self.nodes.lock().await;
        nodes.insert(
            node_data.node_id.clone(),
            NodeState {
                last_value: node_data.clone(),
                last_update: SystemTime::now(),
            },
        );

        let callbacks = self.callbacks.lock().await;
        if let Some(callback) = callbacks.get(&node_data.node_id) {
            let callback = callback.lock().await;
            callback(node_data);
        }
    }

    pub async fn check_node_health(&self) {
        let mut nodes = self.nodes.lock().await;
        for (node_id, node_state) in nodes.iter_mut() {
            let key = format!("node/{}/status", node_id);
            match self.session.get(&key).res().await {
                Ok(receiver) => {
                    match receiver.recv_async().await {
                        Ok(reply) => {
                            if let Ok(sample) = reply.sample {
                                if let Ok(status) =
                                    std::str::from_utf8(&sample.value.payload.contiguous())
                                {
                                    node_state.last_value = NodeData::from_json(status).unwrap();
                                    if node_state.last_value.status != "online" {
                                        warn!("Node {} is offline", node_id);
                                        node_state.last_value.status = "offline".to_string();
                                        // Handle node failure, e.g., update node status, notify subscribers, etc.
                                    }
                                } else {
                                    warn!("Failed to parse status for node {}", node_id);
                                    node_state
                                        .last_value
                                        .set_status("unknown".to_owned())
                                        .map_err(|e| warn!("Failed to set status: {}", e))
                                        .ok();
                                }
                            } else {
                                warn!("No sample available for node {}", node_id);
                                node_state
                                    .last_value
                                    .set_status("unknown".to_owned())
                                    .map_err(|e| warn!("Failed to set status: {}", e))
                                    .ok();
                            }
                        }
                        Err(e) => {
                            warn!("Failed to receive reply for node {}: {}", node_id, e);
                            node_state
                                .last_value
                                .set_status("unknown".to_owned())
                                .map_err(|e| warn!("Failed to set status: {}", e))
                                .ok();
                        }
                    }
                }
                Err(err) => {
                    warn!("Failed to get status for node {}: {}", node_id, err);
                    node_state
                        .last_value
                        .set_status("unknown".to_owned())
                        .map_err(|e| warn!("Failed to set status: {}", e))
                        .ok();
                }
            }
        }
        sleep(Duration::from_secs(1)).await; // Adjust the interval as needed
    }

    pub async fn update_node_config(&self, node_id: &str, config: Value) -> Result<()> {
        let key = format!("fabric/{}/config", node_id);
        let config_json = serde_json::to_string(&config).map_err(FabricError::SerdeJsonError)?;
        let mut backoff = ExponentialBackoff::default();

        loop {
            match self.session.put(&key, config_json.clone()).res().await {
                Ok(_) => return Ok(()),
                Err(err) => {
                    if let Some(duration) = backoff.next_backoff() {
                        warn!(
                            "Failed to update node config, retrying in {:?}: {}",
                            duration, err
                        );
                        sleep(duration).await;
                    } else {
                        return Err(FabricError::Other(format!(
                            "Failed to update node config: {}",
                            err
                        )));
                    }
                }
            }
        }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub async fn register_callback(
        &self,
        node_id: &str,
        callback: Arc<Mutex<dyn Fn(NodeData) + Send + Sync>>,
    ) -> Result<()> {
        let mut callbacks = self.callbacks.lock().await;
        callbacks.insert(node_id.to_string(), callback);
        Ok(())
    }

    async fn check_offline_nodes(&self) {
        let mut nodes = self.nodes.lock().await;
        let now = SystemTime::now();
        for (node_id, node_state) in nodes.iter_mut() {
            if node_state.last_value.status == "online" {
                if let Ok(duration) = now.duration_since(node_state.last_update) {
                    if duration > Duration::from_secs(10) {
                        warn!("Node {} has not sent a status update in 10 seconds, marking as offline", node_id);
                        node_state.last_value.status = "offline".to_string();

                        // Trigger callbacks for the status change
                        let callbacks = self.callbacks.lock().await;
                        if let Some(callback) = callbacks.get(node_id) {
                            let callback = callback.lock().await;
                            callback(node_state.last_value.clone());
                        }
                    }
                }
            }
        }
    }

    pub async fn create_publisher(&self, topic: String) -> Result<()> {
        let key_expr = topic.clone();
        let zenoh_publisher = self
            .session
            .declare_publisher(key_expr)
            .res()
            .await
            .map_err(FabricError::ZenohError)?;

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
                .map_err(FabricError::ZenohError)?;
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
            .map_err(FabricError::ZenohError)?;

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

    pub async fn get_nodes(&self) -> HashMap<String, NodeState> {
        self.nodes.lock().await.clone()
    }
}
