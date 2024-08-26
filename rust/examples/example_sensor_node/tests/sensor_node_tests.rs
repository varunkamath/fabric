use fabric::sensor::{SensorConfig, SensorNode};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sensor_node_creation() {
    let config = zenoh::config::Config::default();
    let session = zenoh::open(config).res().await.unwrap();

    let sensor_config = SensorConfig {
        sensor_id: "test_sensor".to_string(),
        sampling_rate: 5,
        threshold: 50.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 100e6}}),
    };

    let result = SensorNode::new(
        "test_sensor".to_string(),
        "radio".to_string(),
        sensor_config,
        Arc::new(session),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sensor_node_run() {
    let config = zenoh::config::Config::default();
    let session = zenoh::open(config).res().await.unwrap();

    let sensor_config = SensorConfig {
        sensor_id: "test_sensor".to_string(),
        sampling_rate: 1,
        threshold: 50.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 100e6}}),
    };

    let sensor_node = SensorNode::new(
        "test_sensor".to_string(),
        "radio".to_string(),
        sensor_config,
        Arc::new(session),
    )
    .await
    .unwrap();

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    let handle = tokio::spawn(async move {
        sensor_node.run(cancel_clone).await.unwrap();
    });

    // Allow the sensor node to run for a short time
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    cancel.cancel();
    handle.await.unwrap();
}
