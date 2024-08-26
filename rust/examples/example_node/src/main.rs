use fabric::error::Result;
use fabric::node::{interface::NodeConfig, Node};
use std::env;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[tokio::main]
async fn main() -> Result<()> {
    let node_id = env::var("NODE_ID").unwrap_or_else(|_| "example_node".to_string());
    let node_type = env::var("NODE_TYPE").unwrap_or_else(|_| "radio".to_string());

    println!("Starting example node: {}", node_id);

    let config = Config::default();
    let session = Arc::new(zenoh::open(config).res().await?);

    let node_config = NodeConfig {
        node_id: node_id.clone(),
        sampling_rate: 1,
        threshold: 50.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 915.0}}),
    };

    let node = Node::new(node_id.clone(), node_type, node_config, session.clone()).await?;

    let cancel = CancellationToken::new();
    println!("Node {} is running...", node_id);
    node.run(cancel.clone()).await?;

    Ok(())
}
