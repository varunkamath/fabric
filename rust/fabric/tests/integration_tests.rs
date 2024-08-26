use fabric::control::ControlNode;
use fabric::plugins::SensorRegistry;
use fabric::sensor::{SensorConfig, SensorData, SensorNode};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sensor_node_creation_and_run() {
    let config = zenoh::config::Config::default();
    let session = zenoh::open(config).res().await.unwrap();
    let sensor_config = SensorConfig {
        sensor_id: "test_sensor".to_string(),
        sampling_rate: 5,
        threshold: 50.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 100e6, "sample_rate": 2e6, "gain": 20.0, "mode": "receive"}}),
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

    // Allow some time for the sensor node to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    cancel.cancel();
    handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_control_node_creation_and_run() {
    let config = zenoh::config::Config::default();
    let session = zenoh::open(config).res().await.unwrap();

    let control_node = ControlNode::new("test_control".to_string(), Arc::new(session))
        .await
        .unwrap();

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let handle = tokio::spawn(async move {
        control_node.run(cancel_clone).await.unwrap();
    });

    // Allow some time for the control node to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    cancel.cancel();
    handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sensor_config_publication() {
    let config = zenoh::config::Config::default();
    let session = zenoh::open(config).res().await.unwrap();

    let control_node = ControlNode::new("test_control".to_string(), Arc::new(session))
        .await
        .unwrap();

    let sensor_config = SensorConfig {
        sensor_id: "test_sensor".to_string(),
        sampling_rate: 10,
        threshold: 75.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 200e6, "sample_rate": 1e6, "gain": 15.0, "mode": "transmit"}}),
    };

    control_node
        .publish_sensor_config("test_sensor", &sensor_config)
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sensor_data_subscription() {
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let control_node = ControlNode::new("test_control".to_string(), session.clone())
        .await
        .unwrap();

    let received_data = Arc::new(Mutex::new(None));
    let received_data_clone = received_data.clone();

    println!("Setting up subscription...");
    control_node
        .subscribe_to_sensor("test_sensor", move |data| {
            println!("Callback received data: {:?}", data);
            let received_data_clone = received_data_clone.clone();
            tokio::spawn(async move {
                let mut received = received_data_clone.lock().await;
                *received = Some(data);
            });
        })
        .await
        .expect("Failed to subscribe to sensor");

    // Allow some time for the subscription to be set up
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Simulate sending sensor data
    let sensor_data = SensorData {
        sensor_id: "test_sensor".to_string(),
        sensor_type: "radio".to_string(),
        value: 42.0,
        timestamp: 1234567890,
        metadata: None,
    };

    // In a real scenario, this would be published by a sensor node
    // For this test, we're manually publishing the data
    let key = "sensor/data";
    println!("Publishing sensor data...");
    session
        .put(key, serde_json::to_string(&sensor_data).unwrap())
        .res()
        .await
        .unwrap();

    // Allow more time for the data to be processed
    println!("Waiting for data to be processed...");
    for i in 0..20 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let received = received_data.lock().await;
        if received.is_some() {
            println!("Data received after {} iterations", i);
            break;
        }
    }

    let received = received_data.lock().await;
    assert!(received.is_some(), "No data received after 10 seconds");
    if let Some(received_data) = received.as_ref() {
        println!("Received data: {:?}", received_data);
        assert_eq!(received_data.sensor_id, "test_sensor");
        assert_eq!(received_data.value, 42.0);
    }
}

#[test]
fn test_sensor_registry() {
    let registry = SensorRegistry::new();
    let config = SensorConfig {
        sensor_id: "test_sensor".to_string(),
        sampling_rate: 5,
        threshold: 50.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 100e6, "sample_rate": 2e6, "gain": 20.0, "mode": "receive"}}),
    };

    let sensor = registry.create_sensor("radio", config.clone());
    assert!(sensor.is_some());

    let unknown_sensor = registry.create_sensor("unknown", config);
    assert!(unknown_sensor.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multiple_sensors() {
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let control_node = Arc::new(
        ControlNode::new("test_control".to_string(), session.clone())
            .await
            .unwrap(),
    );

    let sensor_configs = vec![
        SensorConfig {
            sensor_id: "sensor1".to_string(),
            sampling_rate: 5,
            threshold: 50.0,
            custom_config: serde_json::json!({"radio_config": {"frequency": 100e6}}),
        },
        SensorConfig {
            sensor_id: "sensor2".to_string(),
            sampling_rate: 10,
            threshold: 75.0,
            custom_config: serde_json::json!({"radio_config": {"frequency": 200e6}}),
        },
    ];

    let received_data = Arc::new(Mutex::new(Vec::new()));

    for config in &sensor_configs {
        let sensor_node = SensorNode::new(
            config.sensor_id.clone(),
            "radio".to_string(),
            config.clone(),
            session.clone(),
        )
        .await
        .unwrap();

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            sensor_node.run(cancel_clone).await.unwrap();
        });

        let received_data_clone = received_data.clone();
        let control_node_clone = control_node.clone();
        control_node_clone
            .subscribe_to_sensor(&config.sensor_id, move |data| {
                let received_data_clone = received_data_clone.clone();
                tokio::spawn(async move {
                    received_data_clone.lock().await.push(data);
                });
            })
            .await
            .unwrap();
    }

    // Allow some time for the sensors to publish data
    tokio::time::sleep(Duration::from_secs(5)).await;

    let received = received_data.lock().await;
    assert!(
        received.len() >= 2,
        "Should receive data from at least 2 sensors"
    );
    assert!(
        received.iter().any(|data| data.sensor_id == "sensor1"),
        "Should receive data from sensor1"
    );
    assert!(
        received.iter().any(|data| data.sensor_id == "sensor2"),
        "Should receive data from sensor2"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sensor_config_update() {
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let initial_config = SensorConfig {
        sensor_id: "test_sensor".to_string(),
        sampling_rate: 5,
        threshold: 50.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 100e6}}),
    };

    let sensor_node = Arc::new(
        SensorNode::new(
            initial_config.sensor_id.clone(),
            "radio".to_string(),
            initial_config.clone(),
            session.clone(),
        )
        .await
        .unwrap(),
    );

    let control_node = ControlNode::new("test_control".to_string(), session.clone())
        .await
        .unwrap();

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let sensor_node_clone = sensor_node.clone();
    tokio::spawn(async move {
        sensor_node_clone.run(cancel_clone).await.unwrap();
    });

    // Allow some time for the sensor node to start
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Update sensor config
    let updated_config = SensorConfig {
        sensor_id: "test_sensor".to_string(),
        sampling_rate: 10,
        threshold: 75.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 200e6}}),
    };

    control_node
        .publish_sensor_config(&updated_config.sensor_id, &updated_config)
        .await
        .unwrap();

    // Allow some time for the config update to be processed
    tokio::time::sleep(Duration::from_secs(2)).await;

    let updated_config = sensor_node.get_config().await;
    assert_eq!(updated_config.sampling_rate, 10);
    assert_eq!(updated_config.threshold, 75.0);

    cancel.cancel();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_error_handling() {
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    // Test handling of unknown sensor type
    let result = SensorNode::new(
        "unknown_sensor".to_string(),
        "unknown_type".to_string(),
        SensorConfig {
            sensor_id: "unknown_sensor".to_string(),
            sampling_rate: 5,
            threshold: 50.0,
            custom_config: serde_json::Value::Null,
        },
        session.clone(),
    )
    .await;

    assert!(
        result.is_err(),
        "Creating an unknown sensor type should fail"
    );

    // Test handling of invalid sensor data
    let invalid_data = r#"{"invalid": "data"}"#;
    let result = session.put("sensor/data", invalid_data).res().await;

    assert!(
        result.is_ok(),
        "Publishing invalid data should not fail, but it should be handled gracefully"
    );

    // Allow some time for error handling
    tokio::time::sleep(Duration::from_secs(2)).await;

    // The test passes if no panics occur during error handling
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_concurrent_operations() {
    let config = zenoh::config::Config::default();
    let session = Arc::new(zenoh::open(config).res().await.unwrap());

    let control_node = Arc::new(
        ControlNode::new("test_control".to_string(), session.clone())
            .await
            .unwrap(),
    );

    let sensor_configs = vec![
        SensorConfig {
            sensor_id: "sensor1".to_string(),
            sampling_rate: 5,
            threshold: 50.0,
            custom_config: serde_json::json!({"radio_config": {"frequency": 100e6}}),
        },
        SensorConfig {
            sensor_id: "sensor2".to_string(),
            sampling_rate: 10,
            threshold: 75.0,
            custom_config: serde_json::json!({"radio_config": {"frequency": 200e6}}),
        },
    ];

    let mut handles = vec![];

    for config in sensor_configs {
        let session_clone = session.clone();
        let control_node_clone = control_node.clone();
        handles.push(tokio::spawn(async move {
            let sensor_node = SensorNode::new(
                config.sensor_id.clone(),
                "radio".to_string(),
                config.clone(),
                session_clone.clone(),
            )
            .await
            .unwrap();

            let cancel = CancellationToken::new();
            let cancel_clone = cancel.clone();
            tokio::spawn(async move {
                sensor_node.run(cancel_clone).await.unwrap();
            });

            // Publish some data
            for _ in 0..5 {
                let data = SensorData {
                    sensor_id: config.sensor_id.clone(),
                    sensor_type: "radio".to_string(),
                    value: rand::random::<f64>() * 100.0,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    metadata: None,
                };
                session_clone
                    .put("sensor/data", serde_json::to_string(&data).unwrap())
                    .res()
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            // Update sensor config
            let updated_config = SensorConfig {
                sensor_id: config.sensor_id.clone(),
                sampling_rate: config.sampling_rate * 2,
                threshold: config.threshold * 1.5,
                custom_config: config.custom_config.clone(),
            };
            control_node_clone
                .publish_sensor_config(&updated_config.sensor_id, &updated_config)
                .await
                .unwrap();

            cancel.cancel();
        }));
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // The test passes if no panics occur during concurrent operations
}
