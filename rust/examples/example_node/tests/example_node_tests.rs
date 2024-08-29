use fabric::error::Result;
use fabric::node::interface::{NodeConfig, NodeInterface};
use fabric::node::Node;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

use example_node::ExampleNode;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_example_node_creation() -> Result<()> {
    let config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({"param1": "test_value", "param2": 100}),
    };

    let example_node = ExampleNode {
        config: config.clone(),
    };

    assert_eq!(example_node.get_config().node_id, config.node_id);
    assert_eq!(example_node.get_type(), "example");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_example_node_read_data() -> Result<()> {
    let config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({}),
    };

    let example_node = ExampleNode { config };

    let data = example_node.read_data().await?;
    assert_eq!(data.node_id, "test_node");
    assert_eq!(data.node_type, "example");
    assert!(data.value >= 0.0 && data.value <= 1.0);
    assert!(data.timestamp > 0);
    assert!(data.metadata.is_some());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_example_node_update_config() -> Result<()> {
    let initial_config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({"param1": "initial_value"}),
    };

    let mut example_node = ExampleNode {
        config: initial_config,
    };

    let new_config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({"param1": "updated_value"}),
    };

    example_node.update_config(new_config.clone());

    assert_eq!(example_node.get_config().node_id, new_config.node_id);
    assert_eq!(example_node.get_config().config, new_config.config);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_example_node_read() -> Result<()> {
    let config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({}),
    };

    let example_node = ExampleNode { config };

    let value = example_node.read().await?;
    assert!(value >= 0.0 && value <= 1.0);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_example_node_handle_event() -> Result<()> {
    let config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({}),
    };

    let mut example_node = ExampleNode { config };

    example_node
        .handle_event("test_topic", r#"{"event_type": "test_event"}"#)
        .await?;

    // Since handle_event is empty in the example, we just check that it doesn't fail
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_node_integration() -> Result<()> {
    let session = zenoh::open(zenoh::config::Config::default()).res().await?;
    let session = Arc::new(session);

    let config = NodeConfig {
        node_id: "test_node".to_string(),
        config: serde_json::json!({"param1": "test_value"}),
    };

    let example_node = ExampleNode {
        config: config.clone(),
    };

    let mut node = Node::new(
        config.node_id.clone(),
        "example".to_string(),
        config,
        session.clone(),
        None,
    )
    .await?;

    node.set_interface(Box::new(example_node)).await?;

    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    let node_task = tokio::spawn(async move { node.run(cancel_token).await });

    // Let the node run for a short time
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Stop the node
    cancel_token_clone.cancel();

    // Wait for the node task to complete
    node_task.await??;

    Ok(())
}
