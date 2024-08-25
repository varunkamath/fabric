use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

// Custom error type
#[derive(Debug)]
struct SensorError(String);

impl fmt::Display for SensorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Sensor error: {}", self.0)
    }
}

impl std::error::Error for SensorError {}

// Add this line to derive Send and Sync
unsafe impl Send for SensorError {}
unsafe impl Sync for SensorError {}

// Define a simple message type for our pub-sub system
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SensorData {
    sensor_id: String,
    value: f64,
}

// Simulated sensor reading function (replace with your actual sensor logic)
async fn read_sensor(sensor_id: String) -> SensorData {
    tokio::time::sleep(Duration::from_secs(1)).await; // Simulate sensor read time
    SensorData {
        sensor_id,
        value: rand::random::<f64>() * 100.0, // Random value between 0 and 100
    }
}

// Publisher function
async fn publish_sensor_data(
    session: Arc<Session>,
    sensor_id: String,
    cancel: CancellationToken,
) -> Result<(), SensorError> {
    let publisher = session
        .declare_publisher("sensor/data")
        .res()
        .await
        .map_err(|e| SensorError(e.to_string()))?;

    while !cancel.is_cancelled() {
        let data = read_sensor(sensor_id.clone()).await;
        let payload = serde_json::to_string(&data).map_err(|e| SensorError(e.to_string()))?;
        println!("Sensor {} publishing data: {:.2}", sensor_id, data.value); // Add this line
        publisher
            .put(payload)
            .res()
            .await
            .map_err(|e| SensorError(e.to_string()))?;
        tokio::time::sleep(Duration::from_secs(5)).await; // Publish every 5 seconds
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct SensorConfig {
    sampling_rate: u64,
    threshold: f64,
    // Add more configuration fields as needed
}

async fn subscribe_to_config(session: Arc<Session>, sensor_id: String) -> Result<(), SensorError> {
    let key = format!("sensor/{}/config", sensor_id);
    let subscriber = session
        .declare_subscriber(&key)
        .res()
        .await
        .map_err(|e| SensorError(e.to_string()))?;

    while let Ok(sample) = subscriber.recv_async().await {
        if let Ok(payload) = std::str::from_utf8(&sample.value.payload.contiguous()) {
            if let Ok(config) = serde_json::from_str::<SensorConfig>(payload) {
                println!(
                    "Received new configuration for sensor {}: {:?}",
                    sensor_id, config
                );
                // Apply the new configuration
                apply_config(config);
            }
        }
    }
    Ok(())
}

fn apply_config(config: SensorConfig) {
    // Apply the configuration to your sensor logic
    // For example, update global variables or reconfigure hardware
    println!("Applying new configuration: {:?}", config);
}

// Remove or comment out the spawn_sensor function
// async fn spawn_sensor(sensor_id: String) -> Result<(), SensorError> {
//     println!("Spawned sensor: {}", sensor_id);

//     let config = zenoh::config::peer();
//     let session = Arc::new(
//         zenoh::open(config)
//             .res()
//             .await
//             .map_err(|e| SensorError(e.to_string()))?,
//     );

//     let cancel = CancellationToken::new();

//     tokio::try_join!(
//         publish_sensor_data(Arc::clone(&session), sensor_id.clone(), cancel.clone()),
//         subscribe_to_config(Arc::clone(&session), sensor_id)
//     )?;

//     Ok(())
// }

#[tokio::main]
async fn main() -> Result<(), SensorError> {
    let sensor_id = env::var("SENSOR_ID").unwrap_or_else(|_| "unknown".to_string());
    let zenoh_peer = env::var("ZENOH_PEER").unwrap_or_else(|_| "tcp/localhost:7447".to_string());

    println!("Starting sensor node with ID: {}", sensor_id);
    println!("Connecting to Zenoh peer: {}", zenoh_peer);

    let mut config = zenoh::config::Config::default();
    config
        .set_mode(Some(zenoh::config::whatami::WhatAmI::Client))
        .unwrap();
    config.connect.endpoints.push(zenoh_peer.parse().unwrap());

    let session = zenoh::open(config)
        .res()
        .await
        .map_err(|e| SensorError(e.to_string()))?;

    let cancel = CancellationToken::new();
    let session = Arc::new(session);

    tokio::select! {
        _ = publish_sensor_data(session.clone(), sensor_id.clone(), cancel.clone()) => {},
        _ = subscribe_to_config(session.clone(), sensor_id.clone()) => {},
        _ = tokio::signal::ctrl_c() => {
            println!("Received Ctrl+C, shutting down...");
            cancel.cancel();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_read_sensor() {
        let sensor_id = "test-sensor".to_string();
        let data = read_sensor(sensor_id.clone()).await;
        assert_eq!(data.sensor_id, sensor_id);
        assert!(data.value >= 0.0 && data.value <= 100.0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_spawn_sensor() {
        let sensor_id = "test-sensor".to_string();
        let result = timeout(Duration::from_secs(5), spawn_sensor(sensor_id.clone())).await;
        assert!(result.is_err(), "spawn_sensor should run indefinitely");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_apply_config() {
        let config = SensorConfig {
            sampling_rate: 10,
            threshold: 75.0,
        };
        apply_config(config);
        // This test just ensures that apply_config doesn't panic
        // In a real scenario, you'd want to check if the configuration was actually applied
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_sensor_error() {
        let error = SensorError("Test error".to_string());
        assert_eq!(error.to_string(), "Sensor error: Test error");
    }
}
