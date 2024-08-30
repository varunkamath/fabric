use fabric::error::{FabricError, Result};
use fabric::node::interface::NodeConfig;
use fabric::node::Node;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

// Import the create_sensor_interface function
use crate::sensors::create_sensor_interface;

const CONFIG_TOPIC: &str = "fabric/config";

#[derive(Debug, Serialize, Deserialize)]
struct SensorConfig {
    id: String,
    #[serde(rename = "type")]
    sensor_type: String,
    config: serde_json::Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize Zenoh session
    let session = Arc::new(zenoh::open(zenoh::config::Config::default()).res().await?);

    // Request configuration
    let node_id = "example_node".to_string(); // Replace with actual node ID
    session.put(CONFIG_TOPIC, node_id.clone()).res().await?;

    // Wait for configuration
    let subscriber = session.declare_subscriber(CONFIG_TOPIC).res().await?;
    let sample = subscriber.recv_async().await?;
    let sensor_config: SensorConfig = serde_json::from_slice(&sample.value.payload.contiguous())?;

    // Create node configuration
    let node_config = NodeConfig {
        node_id: sensor_config.id,
        config: sensor_config.config,
    };

    // Create the node
    let mut node = Node::new(
        node_config.node_id.clone(),
        sensor_config.sensor_type.clone(),
        node_config.clone(),
        session.clone(),
        None,
    )
    .await?;

    // Set the node interface using the create_sensor_interface function
    let sensor_interface = create_sensor_interface(&sensor_config.sensor_type, node_config);
    node.set_interface(sensor_interface).await?;

    // Run the node
    let cancel_token = CancellationToken::new();
    node.run(cancel_token).await?;

    Ok(())
}
