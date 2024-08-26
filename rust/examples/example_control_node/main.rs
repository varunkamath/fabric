use fabric::control::{ControlConfig, ControlNode};
use fabric::sensor::interface::SensorConfig;
use std::env;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let control_id = env::var("CONTROL_ID").unwrap_or_else(|_| "example_control".to_string());
    let zenoh_peer = env::var("ZENOH_PEER").unwrap_or_else(|_| "tcp/localhost:7447".to_string());

    println!("Starting control node with ID: {}", control_id);
    println!("Connecting to Zenoh peer: {}", zenoh_peer);

    let mut config = fabric::zenoh::config::Config::default();
    config
        .set_mode(Some(fabric::zenoh::config::whatami::WhatAmI::Client))
        .unwrap();
    config.connect.endpoints.push(zenoh_peer.parse().unwrap());

    let session = fabric::zenoh::open(config).res().await?;

    let control_node = ControlNode::new(control_id, Arc::new(session)).await?;
    let cancel = CancellationToken::new();

    // Example configuration
    let control_config = ControlConfig {
        sensors: vec![SensorConfig {
            sensor_id: "example_sensor".to_string(),
            sampling_rate: 5,
            threshold: 50.0,
            custom_config: serde_json::json!({"radio_config": {"frequency": 100e6, "sample_rate": 2e6, "gain": 20.0, "mode": "receive"}}),
        }],
    };

    // Publish sensor configurations
    control_node.publish_sensor_configs(&control_config).await?;

    // Subscribe to all sensors
    control_node
        .subscribe_to_sensor("sensor/**", |data| {
            println!(
                "Received data from sensor {}: {:.2}",
                data.sensor_id, data.value
            );
            // Add your custom logic here
        })
        .await;

    tokio::select! {
        result = control_node.run(cancel.clone()) => {
            if let Err(e) = result {
                eprintln!("Control node error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("Received Ctrl+C, shutting down...");
            cancel.cancel();
        }
    }

    Ok(())
}
