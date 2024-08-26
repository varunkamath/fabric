use fabric::error::Result;
use fabric::node::{interface::NodeConfig, Node};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[tokio::test]
async fn test_node_creation_and_run() -> Result<()> {
    let node_id = "test_node".to_string();
    let node_type = "radio".to_string();

    let config = Config::default();
    let session = Arc::new(zenoh::open(config).res().await?);

    let node_config = NodeConfig {
        node_id: node_id.clone(),
        sampling_rate: 1,
        threshold: 50.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 915.0}, "mode": "IDLE"}),
    };

    let node = Node::new(
        node_id.clone(),
        node_type,
        node_config.clone(),
        session.clone(),
    )
    .await?;

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    let handle = tokio::spawn(async move {
        node.run(cancel_clone).await.unwrap();
    });

    // Allow some time for the node to start
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Verify initial config
    let initial_config = node.get_config().await;
    assert_eq!(initial_config.sampling_rate, 1);
    assert_eq!(initial_config.threshold, 50.0);
    assert_eq!(initial_config.custom_config["mode"], "IDLE");

    // Simulate orchestrator sending new config
    let new_config = NodeConfig {
        node_id: node_id.clone(),
        sampling_rate: 5,
        threshold: 75.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 2400.0}, "mode": "ACTIVE"}),
    };

    session
        .put(
            format!("node/{}/config", node_id),
            serde_json::to_string(&new_config)?,
        )
        .res()
        .await?;

    // Allow some time for the config update to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Verify updated config
    let updated_config = node.get_config().await;
    assert_eq!(updated_config.sampling_rate, 5);
    assert_eq!(updated_config.threshold, 75.0);
    assert_eq!(updated_config.custom_config["mode"], "ACTIVE");

    cancel.cancel();
    handle.await.unwrap();

    Ok(())
}
