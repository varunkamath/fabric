use fabric::control::ControlNode;
use fabric::sensor::interface::SensorConfig;
use fabric::sensor::SensorData;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_control_node_creation() {
    let config = zenoh::config::Config::default();
    let session = zenoh::open(config).res().await.unwrap();

    let result = ControlNode::new("test_control".to_string(), Arc::new(session)).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_control_node_run() {
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let control_node = ControlNode::new("test_control".to_string(), session.clone())
        .await
        .unwrap();

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    let handle = tokio::spawn(async move {
        control_node.run(cancel_clone).await.unwrap();
    });

    // Allow the control node to run for a short time
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    cancel.cancel();
    handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_publish_sensor_config() {
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let control_node = ControlNode::new("test_control".to_string(), session.clone())
        .await
        .unwrap();

    let sensor_config = SensorConfig {
        sensor_id: "test_sensor".to_string(),
        sampling_rate: 5,
        threshold: 50.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 100e6}}),
    };

    let result = control_node
        .publish_sensor_config("test_sensor", &sensor_config)
        .await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_subscribe_to_sensor() {
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let control_node = ControlNode::new("test_control".to_string(), session.clone())
        .await
        .unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    control_node
        .subscribe_to_sensor("test_sensor", move |data| {
            let tx = tx.clone();
            tokio::spawn(async move {
                tx.send(data).await.unwrap();
            });
        })
        .await
        .unwrap();

    // Simulate sending sensor data
    let sensor_data = SensorData {
        sensor_id: "test_sensor".to_string(),
        sensor_type: "radio".to_string(),
        value: 42.0,
        timestamp: 1234567890,
        metadata: None,
    };

    session
        .put("sensor/data", serde_json::to_string(&sensor_data).unwrap())
        .res()
        .await
        .unwrap();

    // Wait for the data to be received
    let received_data = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(received_data.sensor_id, "test_sensor");
    assert_eq!(received_data.value, 42.0);
}
