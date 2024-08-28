use fabric::error::Result;
use fabric::node::{interface::NodeConfig, Node};
use fabric::init_logger;
use log::{info, warn};
use std::env;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger(log::LevelFilter::Info);

    let node_id = env::var("NODE_ID").unwrap_or_else(|_| "example_node".to_string());
    let node_type = env::var("NODE_TYPE").unwrap_or_else(|_| "generic".to_string());

    info!("Starting example node: {}", node_id);

    let config = Config::default();
    let session = Arc::new(zenoh::open(config).res().await?);

    let node_config = NodeConfig {
        node_id: node_id.clone(),
        config: serde_json::json!({
            "sampling_rate": 1,
            "threshold": 50.0
        }),
    };

    let node = Node::new(node_id.clone(), node_type, node_config, session.clone()).await?;

    // Subscribe to another node
    node.subscribe_to_node("other_node", |data| {
        println!("Received data from other_node: {:?}", data);
    }).await?;

    // Subscribe to a custom topic
    node.subscribe_to_topic("custom/topic", |sample| {
        println!("Received data on custom topic: {:?}", sample);
    }).await?;

    let cancel = CancellationToken::new();
    println!("Node {} is running...", node_id);
    node.run(cancel.clone()).await?;

    // Unsubscribe when done
    node.unsubscribe_from_node("other_node").await?;
    node.unsubscribe_from_topic("custom/topic").await?;

    Ok(())
}