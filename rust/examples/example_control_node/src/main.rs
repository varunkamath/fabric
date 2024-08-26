use fabric::control::ControlNode;
use fabric::error::{FabricError, Result};
use fabric::sensor::interface::SensorConfig;
use std::env;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use zenoh::prelude::r#async::*;

#[tokio::main]
async fn main() -> Result<()> {
    let control_id = env::var("CONTROL_ID").unwrap_or_else(|_| "example_control".to_string());
    let zenoh_peer = env::var("ZENOH_PEER").unwrap_or_else(|_| "tcp/localhost:7447".to_string());

    println!("Starting control node with ID: {}", control_id);
    println!("Connecting to Zenoh peer: {}", zenoh_peer);

    let mut config = zenoh::config::Config::default();
    config
        .set_mode(Some(zenoh::config::whatami::WhatAmI::Client))
        .unwrap();
    config.connect.endpoints.push(zenoh_peer.parse().unwrap());

    let session = zenoh::open(config)
        .res()
        .await
        .map_err(FabricError::ZenohError)?;

    let control_node = ControlNode::new(control_id, Arc::new(session)).await?;
    let cancel = CancellationToken::new();

    // Example configurations for multiple sensors
    let sensor_configs = vec![
        SensorConfig {
            sensor_id: "radio1".to_string(),
            sampling_rate: 1,
            threshold: 0.0,
            custom_config: serde_json::json!({
                "mode": "receive",
                "receive_config": {
                    "frequency": 100e6,
                    "sample_rate": 2e6,
                    "gain": 20.0
                }
            }),
        },
        SensorConfig {
            sensor_id: "radio2".to_string(),
            sampling_rate: 1,
            threshold: 0.0,
            custom_config: serde_json::json!({
                "mode": "transmit",
                "transmit_config": {
                    "frequency": 100e6,
                    "sample_rate": 2e6,
                    "gain": 10.0,
                    "tx_power": 0.1
                }
            }),
        },
    ];

    // Publish initial sensor configurations
    for sensor_config in &sensor_configs {
        control_node
            .publish_sensor_config(&sensor_config.sensor_id, sensor_config)
            .await?;
    }

    // Subscribe to all sensors
    control_node
        .subscribe_to_sensor("sensor/**", |data| {
            println!(
                "Received data from sensor {}: {:.2} ({})",
                data.sensor_id, data.value, data.sensor_type
            );
        })
        .await?;

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
        _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
            // After 30 seconds, change configurations
            println!("Changing sensor configurations...");
            let new_configs = vec![
                SensorConfig {
                    sensor_id: "radio1".to_string(),
                    sampling_rate: 2,
                    threshold: 0.0,
                    custom_config: serde_json::json!({
                        "mode": "transmit",
                        "transmit_config": {
                            "frequency": 101e6,
                            "sample_rate": 1e6,
                            "gain": 15.0,
                            "tx_power": 0.2
                        }
                    }),
                },
                SensorConfig {
                    sensor_id: "radio2".to_string(),
                    sampling_rate: 2,
                    threshold: 0.0,
                    custom_config: serde_json::json!({
                        "mode": "receive",
                        "receive_config": {
                            "frequency": 101e6,
                            "sample_rate": 1e6,
                            "gain": 25.0
                        }
                    }),
                },
            ];

            for new_config in &new_configs {
                control_node.publish_sensor_config(&new_config.sensor_id, new_config).await?;
            }
        }
    }

    Ok(())
}
