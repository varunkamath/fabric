use fabric::error::Result;
use fabric::node::{interface::NodeConfig, Node, NodeData};
use std::sync::Arc;
use tokio::sync::Mutex;
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
        config: serde_json::json!({
            "sampling_rate": 1,
            "threshold": 50.0,
            "radio_config": {"frequency": 915.0},
            "mode": "IDLE"
        }),
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
    assert_eq!(initial_config.config["sampling_rate"], 1);
    assert_eq!(initial_config.config["threshold"], 50.0);
    assert_eq!(initial_config.config["mode"], "IDLE");

    // Simulate orchestrator sending new config
    let new_config = NodeConfig {
        node_id: node_id.clone(),
        config: serde_json::json!({
            "sampling_rate": 5,
            "threshold": 75.0,
            "radio_config": {"frequency": 2400.0},
            "mode": "ACTIVE"
        }),
    };

    session
        .put(
            format!("node/{}/config", node_id),
            serde_json::to_string(&new_config).unwrap(),
        )
        .res()
        .await?;

    // Allow some time for the config to be updated
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Verify updated config
    let updated_config = node.get_config().await;
    assert_eq!(updated_config.config["sampling_rate"], 5);
    assert_eq!(updated_config.config["threshold"], 75.0);
    assert_eq!(updated_config.config["mode"], "ACTIVE");

    cancel.cancel();
    handle.await.unwrap();

    Ok(())
}

#[tokio::test]
async fn test_node_subscribe_to_node() -> Result<()> {
    let config = Config::default();
    let session = Arc::new(zenoh::open(config).res().await?);

    let node1 = Node::new(
        "node1".to_string(),
        "test".to_string(),
        NodeConfig {
            node_id: "node1".to_string(),
            sampling_rate: 1,
            threshold: 50.0,
            custom_config: serde_json::json!({}),
        },
        session.clone(),
    )
    .await?;

    let node2 = Node::new(
        "node2".to_string(),
        "test".to_string(),
        NodeConfig {
            node_id: "node2".to_string(),
            sampling_rate: 1,
            threshold: 50.0,
            custom_config: serde_json::json!({}),
        },
        session.clone(),
    )
    .await?;

    let received_data = Arc::new(Mutex::new(Vec::new()));
    let received_data_clone = received_data.clone();

    node1
        .subscribe_to_node("node2", move |data| {
            let received_data_clone = received_data_clone.clone();
            tokio::spawn(async move {
                received_data_clone.lock().await.push(data);
            });
        })
        .await?;

    // Simulate node2 publishing data
    let data = NodeData {
        node_id: "node2".to_string(),
        node_type: "test".to_string(),
        value: 42.0,
        timestamp: 1234567890,
        metadata: None,
    };
    session
        .put("node/node2/data", serde_json::to_string(&data)?)
        .res()
        .await?;

    // Allow some time for the data to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let received = received_data.lock().await;
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].node_id, "node2");
    assert_eq!(received[0].value, 42.0);

    Ok(())
}

#[tokio::test]
async fn test_node_subscribe_to_topic() -> Result<()> {
    let config = Config::default();
    let session = Arc::new(zenoh::open(config).res().await?);

    let node = Node::new(
        "test_node".to_string(),
        "test".to_string(),
        NodeConfig {
            node_id: "test_node".to_string(),
            sampling_rate: 1,
            threshold: 50.0,
            custom_config: serde_json::json!({}),
        },
        session.clone(),
    )
    .await?;

    let received_data = Arc::new(Mutex::new(Vec::new()));
    let received_data_clone = received_data.clone();

    node.subscribe_to_topic("test/topic", move |sample| {
        let received_data_clone = received_data_clone.clone();
        tokio::spawn(async move {
            if let Ok(value) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                received_data_clone.lock().await.push(value.to_string());
            }
        });
    })
    .await?;

    // Publish data to the topic
    session.put("test/topic", "test_data").res().await?;

    // Allow some time for the data to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let received = received_data.lock().await;
    assert_eq!(received.len(), 1);
    assert_eq!(received[0], "test_data");

    Ok(())
}

#[tokio::test]
async fn test_node_unsubscribe() -> Result<()> {
    let config = Config::default();
    let session = Arc::new(zenoh::open(config).res().await?);

    let node = Node::new(
        "test_node".to_string(),
        "test".to_string(),
        NodeConfig {
            node_id: "test_node".to_string(),
            sampling_rate: 1,
            threshold: 50.0,
            custom_config: serde_json::json!({}),
        },
        session.clone(),
    )
    .await?;

    let received_data = Arc::new(Mutex::new(Vec::new()));
    let received_data_clone = received_data.clone();

    node.subscribe_to_topic("test/topic", move |sample| {
        let received_data_clone = received_data_clone.clone();
        tokio::spawn(async move {
            if let Ok(value) = std::str::from_utf8(&sample.value.payload.contiguous()) {
                received_data_clone.lock().await.push(value.to_string());
            }
        });
    })
    .await?;

    // Publish data to the topic
    session.put("test/topic", "test_data_1").res().await?;

    // Allow some time for the data to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Unsubscribe from the topic
    node.unsubscribe_from_topic("test/topic").await?;

    // Publish more data to the topic
    session.put("test/topic", "test_data_2").res().await?;

    // Allow some time for the data to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let received = received_data.lock().await;
    assert_eq!(received.len(), 1);
    assert_eq!(received[0], "test_data_1");

    Ok(())
}
