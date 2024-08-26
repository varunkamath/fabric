use fabric::sensor::{SensorConfig, SensorNode};
use std::env;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let sensor_id = env::var("SENSOR_ID").unwrap_or_else(|_| "example_sensor".to_string());
    let zenoh_peer = env::var("ZENOH_PEER").unwrap_or_else(|_| "tcp/localhost:7447".to_string());

    println!("Starting radio sensor node with ID: {}", sensor_id);
    println!("Connecting to Zenoh peer: {}", zenoh_peer);

    let mut config = zenoh::config::Config::default();
    config
        .set_mode(Some(zenoh::config::whatami::WhatAmI::Client))
        .unwrap();
    config.connect.endpoints.push(zenoh_peer.parse().unwrap());

    let session = zenoh::open(config).res().await?;

    let initial_config = SensorConfig {
        sensor_id: sensor_id.clone(),
        sampling_rate: 1, // Default to 1 second
        threshold: 0.0,   // Not used for radio
        custom_config: serde_json::json!({
            "mode": "idle",
            "idle_config": {},
            "receive_config": {
                "frequency": 100e6,
                "sample_rate": 2e6,
                "gain": 20.0
            },
            "transmit_config": {
                "frequency": 100e6,
                "sample_rate": 2e6,
                "gain": 10.0,
                "tx_power": 0.1
            }
        }),
    };

    let sensor_node = SensorNode::new(
        sensor_id,
        "radio".to_string(),
        initial_config,
        Arc::new(session),
    )
    .await?;

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
