use super::interface::{NodeConfig, NodeData, NodeInterface};
use crate::error::{FabricError, Result};
use crate::plugins;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

pub struct Node {
    id: String,
    interface: Arc<Mutex<Box<dyn NodeInterface>>>,
    session: Arc<Session>,
}

impl Node {
    pub async fn new(
        id: String,
        node_type: String,
        config: NodeConfig,
        session: Arc<Session>,
    ) -> Result<Self> {
        let interface = plugins::create_node(&node_type, config)
            .ok_or_else(|| FabricError::Other(format!("Unknown node type: {}", node_type)))?;

        Ok(Self {
            id,
            interface: Arc::new(Mutex::new(interface)),
            session,
        })
    }

    pub async fn run(&self, cancel: CancellationToken) -> Result<()> {
        let publisher = self
            .session
            .declare_publisher("node/data")
            .res()
            .await
            .map_err(FabricError::ZenohError)?;

        let config_subscriber = self
            .session
            .declare_subscriber(&format!("node/{}/config", self.id))
            .res()
            .await
            .map_err(FabricError::ZenohError)?;

        let event_subscriber = self
            .session
            .declare_subscriber(&format!("node/{}/event/*", self.id))
            .res()
            .await
            .map_err(FabricError::ZenohError)?;

        let mut last_publish = Instant::now();
        let mut sampling_interval = Duration::from_secs(5); // Default interval

        while !cancel.is_cancelled() {
            tokio::select! {
                _ = tokio::time::sleep_until(last_publish + sampling_interval) => {
                    let node_value = {
                        let interface = self.interface.lock().await;
                        interface.read().await?
                    };

                    let node_data = NodeData {
                        node_id: self.id.clone(),
                        node_type: {
                            let interface = self.interface.lock().await;
                            interface.get_type()
                        },
                        value: node_value,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        metadata: None,
                    };

                    let data_json = serde_json::to_string(&node_data)?;
                    publisher.put(data_json).res().await.map_err(FabricError::ZenohError)?;
                    println!("Published node data: {:?}", node_data);

                    last_publish = Instant::now();
                }

                Ok(sample) = config_subscriber.recv_async() => {
                    if let Ok(config_json) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                        if let Ok(new_config) = serde_json::from_str::<NodeConfig>(config_json) {
                            println!("Received new configuration: {:?}", new_config);
                            let mut interface = self.interface.lock().await;
                            interface.set_config(new_config.clone());
                            sampling_interval = Duration::from_secs(new_config.sampling_rate);
                        }
                    }
                }

                Ok(sample) = event_subscriber.recv_async() => {
                    if let Ok(payload) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                        let key_expr = sample.key_expr.as_str();
                        if let Some(event) = key_expr.split('/').last() {
                            println!("Received event for node {}: {} - {}", self.id, event, payload);
                            let mut interface = self.interface.lock().await;
                            if let Err(e) = interface.handle_event(event, payload).await {
                                eprintln!("Error handling event: {}", e);
                            }
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

    pub async fn get_config(&self) -> NodeConfig {
        let interface = self.interface.lock().await;
        interface.get_config()
    }
}
