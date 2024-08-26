use fabric::sensor::{SensorConfig, SensorNode};
use std::env;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sensor_id = env::var("SENSOR_ID").unwrap_or_else(|_| "example_sensor".to_string());
    let sensor_type = env::var("SENSOR_TYPE").unwrap_or_else(|_| "radio".to_string());
    let zenoh_peer = env::var("ZENOH_PEER").unwrap_or_else(|_| "tcp/localhost:7447".to_string());

    println!(
        "Starting sensor node with ID: {} and type: {}",
        sensor_id, sensor_type
    );
    println!("Connecting to Zenoh peer: {}", zenoh_peer);

    let mut config = fabric::zenoh::config::Config::default();
    config
        .set_mode(Some(fabric::zenoh::config::whatami::WhatAmI::Client))
        .unwrap();
    config.connect.endpoints.push(zenoh_peer.parse().unwrap());

    let session = fabric::zenoh::open(config).res().await?;

    let initial_config = SensorConfig {
        sensor_id: sensor_id.clone(),
        sampling_rate: 5,
        threshold: 50.0,
        custom_config: serde_json::json!({"radio_config": {"frequency": 100e6, "sample_rate": 2e6, "gain": 20.0, "mode": "receive"}}),
    };

    let sensor_node =
        SensorNode::new(sensor_id, sensor_type, initial_config, Arc::new(session)).await?;
    let cancel = CancellationToken::new();

    tokio::select! {
        result = sensor_node.run(cancel.clone()) => {
            if let Err(e) = result {
                eprintln!("Sensor node error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("Received Ctrl+C, shutting down...");
            cancel.cancel();
        }
    }

    Ok(())
}
